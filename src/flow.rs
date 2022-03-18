use std::cmp;

use crate::{
    packet::Ack,
    time::Time,
    units::{BitsPerSec, Bytes, Nanosecs},
    Packet, SourceId,
};

identifier!(FlowId);

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub(crate) struct Flow {
    pub(crate) id: FlowId,
    source: SourceId,
    size: Bytes,
    #[builder(setter(into))]
    src2btl: Nanosecs,
    #[builder(setter(into))]
    btl2dst: Nanosecs,

    // Rate management
    #[builder(setter(into))]
    rate: BitsPerSec,
    #[builder(default = BitsPerSec::new(1_000_000_000))]
    min_rate: BitsPerSec,
    #[builder(setter(into))]
    max_rate: BitsPerSec,
    pub(crate) tnext: Time,

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
}

impl Flow {
    pub(crate) fn bytes_left(&self) -> Bytes {
        self.size.saturating_sub(self.snd_nxt)
    }

    pub(crate) fn on_the_fly(&self) -> Bytes {
        self.snd_nxt - self.snd_una
    }

    pub(crate) fn variable_window(&self) -> Bytes {
        self.window
            .scale_by(BitsPerSec::frac(self.rate, self.max_rate))
    }

    pub(crate) fn usable_window(&self) -> Bytes {
        self.variable_window().saturating_sub(self.on_the_fly())
    }

    pub(crate) fn is_rate_bound(&self, now: Time) -> bool {
        self.tnext > now
    }

    pub(crate) fn is_win_bound(&self) -> bool {
        self.usable_window() == Bytes::ZERO
    }

    pub(crate) fn next_packet(&mut self, now: Time) -> Packet {
        assert!(self.bytes_left() > Bytes::ZERO);
        assert!(self.usable_window() > Bytes::ZERO);

        // Amount to send is capped by the remaining flow size, the maximum packet size, and the
        // usable window size.
        let sz_payload = cmp::min(self.bytes_left(), Packet::SZ_MAX);
        let sz_payload = cmp::min(sz_payload, self.usable_window());
        self.snd_nxt += sz_payload;
        let sz_pkt = sz_payload + Packet::SZ_HDR;
        let rate_delta = self.rate.length(sz_pkt).into_delta();
        self.tnext = now + rate_delta;

        let is_last = self.bytes_left() == Bytes::ZERO;
        Packet::builder()
            .flow_id(self.id)
            .source_id(self.source)
            .size(sz_pkt)
            .is_last(is_last)
            .src2btl(self.src2btl)
            .btl2dst(self.btl2dst)
            .build()
    }

    // TODO: update `tnext`
    pub(crate) fn rcv_ack(&mut self, ack: Ack) {
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
                let new_rate = self.rate.saturating_add(self.additive_inc);
                self.rate = cmp::min(self.max_rate, new_rate);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, derivative::Derivative)]
#[derivative(Default)]
enum CaState {
    #[derivative(Default)]
    Zero,
    One,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct FlowDesc {
    pub id: FlowId,
    pub source: SourceId,
    pub size: Bytes,
    pub start: Nanosecs,
    pub delay2dst: Nanosecs, // propagation delay to destination
}
