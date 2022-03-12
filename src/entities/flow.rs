use std::cmp;

use crate::{
    packet::{Ack, Packet},
    simulation::{event::EventList, Context},
    time::Time,
    units::{BitsPerSec, Bytes, Nanosecs},
};

use super::bottleneck::BottleneckCmd;

entity_id!(FlowId);

#[derive(Debug, Clone, Copy, typed_builder::TypedBuilder)]
pub(crate) struct Flow {
    pub(crate) id: FlowId,
    #[builder(setter(into))]
    pub(crate) size: Bytes,
    #[builder(setter(into))]
    pub(crate) start: Nanosecs,
    #[builder(setter(into))]
    pub(crate) src2btl: Nanosecs,
    #[builder(setter(into))]
    pub(crate) btl2dst: Nanosecs,

    // Rate management
    #[builder(setter(into))]
    rate: BitsPerSec,
    #[builder(default = BitsPerSec::new(100_000_000))]
    min_rate: BitsPerSec,
    #[builder(setter(into))]
    pub(crate) max_rate: BitsPerSec,

    // Window management
    #[builder(setter(into))]
    window: Bytes,
    #[builder(default, setter(skip))]
    snd_nxt: Bytes,
    #[builder(default, setter(skip))]
    snd_una: Bytes,

    // DCTCP
    #[builder(default = 1.0, setter(skip))]
    alpha: f64,
    gain: f64,
    #[builder(setter(into))]
    additive_inc: BitsPerSec,
    #[builder(default, setter(skip))]
    last_update_seq: Bytes,
    #[builder(default, setter(skip))]
    batch_size: usize,
    #[builder(default, setter(skip))]
    marked_count: usize,
    #[builder(default, setter(skip))]
    ca_state: CaState,
    #[builder(default, setter(skip))]
    high_seq: Bytes,

    // Event management
    #[builder(default, setter(skip))]
    version: u128,
    #[builder(default, setter(skip))]
    pending: Pending,
    #[builder(default, setter(skip))]
    is_paused: bool,
}

impl Flow {
    fn bytes_left(&self) -> Bytes {
        self.size.saturating_sub(self.snd_nxt)
    }

    fn on_the_fly(&self) -> Bytes {
        self.snd_nxt - self.snd_una
    }

    fn usable_window(&self) -> Bytes {
        self.window.saturating_sub(self.on_the_fly())
    }
}

impl Flow {
    #[must_use]
    pub(crate) fn step(&mut self, version: u128, mut ctx: Context) -> EventList {
        let now = ctx.cur_time;
        if version != self.version {
            // This step event is no longer valid
            return ctx.into_events();
        }
        if self.bytes_left() == Bytes::ZERO {
            return ctx.into_events();
        }
        if self.usable_window() < Packet::SZ_MIN {
            // There is not enough window to send even a single packet
            self.is_paused = true;
            return ctx.into_events();
        }

        // Amount to send is capped by the remaining flow size, the maximum packet size, and the
        // usable window size.
        let sz_payload = cmp::min(self.bytes_left(), Packet::SZ_MAX);
        let sz_payload = cmp::min(sz_payload, self.usable_window());
        // This needs to happen before the payload is padded
        self.snd_nxt += sz_payload;

        // Pad the payload up to at least `SZ_MIN` bytes
        let sz_pkt = cmp::max(sz_payload, Packet::SZ_MIN);
        let is_last = self.bytes_left() == Bytes::ZERO;

        let pkt = Packet::builder()
            .flow_id(self.id)
            .size(sz_pkt)
            .is_last(is_last)
            .src2btl(self.src2btl)
            .btl2dst(self.btl2dst)
            .build();

        // Send the packet to the bottleneck
        let bw_delta = self.rate.length(pkt.size).into_delta();
        ctx.schedule(
            self.src2btl.into_delta() + bw_delta,
            BottleneckCmd::new_receive(pkt),
        );

        if !is_last {
            // Reschedule the flow
            ctx.schedule(bw_delta, FlowCmd::new_step(self.id, self.version));
            self.pending = Pending::new(now, pkt.size, self.rate, now + bw_delta);
        }

        ctx.into_events()
    }

    #[must_use]
    pub(crate) fn rcv_ack(&mut self, ack: Ack, ctx: Context) -> EventList {
        self.snd_una += ack.nr_bytes;
        let mut new_batch = false;
        if ack.marked {
            self.marked_count += 1;
        }
        // Update alpha
        if self.snd_una > self.last_update_seq {
            new_batch = true;
            if self.last_update_seq == Bytes::ZERO {
                // First RTT
                self.batch_size = Packet::max_count_in(self.snd_nxt);
            } else {
                let frac = (self.marked_count as f64 / self.batch_size as f64).clamp(0.0, 1.0);
                self.alpha = (1.0 - self.gain) * self.alpha + self.gain * frac;
                self.marked_count = 0;
                self.batch_size = Packet::max_count_in(self.snd_nxt - self.snd_una);
            }
            self.last_update_seq = self.snd_nxt;
        }

        if self.ca_state == CaState::One && self.snd_una > self.high_seq {
            self.ca_state = CaState::Zero;
        }
        if self.ca_state == CaState::Zero {
            if ack.marked {
                // Reduce rate
                let new_rate = self.rate.scale_by(1.0 - self.alpha / 2.0);
                self.rate = cmp::max(self.min_rate, new_rate);
                self.ca_state = CaState::One;
                self.high_seq = self.snd_nxt;
            }
            if new_batch {
                let new_rate = self.rate + self.additive_inc;
                self.rate = cmp::min(self.max_rate, new_rate);
            }
        }
        // TODO: In theory, any pending send event should be cancelled and rescheduled, but I'm not
        // sure the HPCC code base does this?? So we'll try to match it for now.
        if self.is_paused {
            self.is_paused = false;
            self.step(self.version, ctx)
        } else {
            ctx.into_events()
        }
    }

    // #[must_use]
    // pub(crate) fn update_rate(&mut self, rate: BitsPerSec, mut ctx: Context) -> EventList {
    //     if rate == self.pending.rate {
    //         // Nothing changed; nothing to do
    //         return ctx.into_events();
    //     }
    //     let now = ctx.cur_time;
    //     self.rate = rate;
    //     if rate == BitsPerSec::ZERO {
    //         // If the new rate is zero, we need to cancel any pending step
    //         self.version += 1;
    //         self.pending = Pending::zero_rate(now);
    //         self.is_paused = true;
    //     } else if !self.is_paused {
    //         // There is a pending step (we're paused), but the rate has changed so we will need to
    //         // reschedule it
    //         let consumed = self.pending.rate.width(now - self.pending.time);
    //         let remaining = self
    //             .pending
    //             .size
    //             .checked_sub(consumed)
    //             .unwrap_or(Bytes::ZERO);
    //         let delta = rate.length(remaining);
    //         let next = now + delta;
    //         if next != self.pending.next {
    //             // If the computed step time has actually changed
    //             self.version += 1;
    //             ctx.schedule(delta, FlowCmd::new_step(self.id, self.version));
    //             self.pending = Pending::new(now, remaining, rate, next);
    //         }
    //     } else if self.should_unpause() {
    //         ctx.schedule_now(FlowCmd::new_step(self.id, self.version));
    //         self.is_paused = false;
    //     }
    //     ctx.into_events()
    // }
}

#[derive(Debug, Copy, Clone, derive_new::new)]
pub(crate) enum FlowCmd {
    Step { id: FlowId, version: u128 },
    RcvAck { id: FlowId, ack: Ack },
}

#[derive(Debug, Default, Copy, Clone, derive_new::new)]
#[allow(unused)]
struct Pending {
    time: Time,
    size: Bytes,
    rate: BitsPerSec,
    next: Time,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, derivative::Derivative)]
#[derivative(Default)]
enum CaState {
    #[derivative(Default)]
    Zero,
    One,
}
