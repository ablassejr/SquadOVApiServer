use crate::{
    SquadOvError,
    combatlog::agg::{
        CombatLogAggregator,
        InputAggregatorPacket,
        OutputAggregatorPacket,
    },
};
use std::{
    time::Duration,
    ops::Range,
};
use chrono::{DateTime, Utc};

pub enum SlidingWindowFunction {
    Average,
    PerUnitTime(Duration),
}

pub struct CombatLogSlidingWindowAggregator<T> {
    func: SlidingWindowFunction,
    window_size: Duration,
    buffer: Vec<T>,
    buffer_range: Range<DateTime<Utc>>,
}

impl<T> CombatLogSlidingWindowAggregator<T>
where
    T: num::traits::Zero + std::ops::Div<Output = T> + num::traits::NumCast + num::traits::ToPrimitive + Copy + std::fmt::Debug,
{
    pub fn new(func: SlidingWindowFunction, window_size: Duration, next_start_time: DateTime<Utc>) -> Self{
        Self {
            func,
            window_size: window_size.clone(),
            buffer: vec![],
            buffer_range: Range{
                start: next_start_time.clone(),
                end: next_start_time + chrono::Duration::from_std(window_size).unwrap(),
            },
        }
    }

    fn clear_buffer(&mut self) {
        self.buffer.clear();
        self.buffer_range = Range{
            start: self.buffer_range.end,
            end: self.buffer_range.end + chrono::Duration::from_std(self.window_size).unwrap(),
        };
    }

    fn compute_next_value_from_buffer(&self, buffer: &[T]) -> Result<T, SquadOvError> {
        // Two steps here:
        // 1) Reduce (going from the buffer to a single value)
        // 2) Final (any modifications we need to do to that final value, e.g. dividing by the total # of elements)
        let reduced_value = buffer.iter()
            .fold(T::zero(), |acc, x| {
                match self.func {
                    _ => acc + *x,
                }
            });

        let val = reduced_value.to_f64().ok_or(SquadOvError::BadRequest)?;
        Ok(
            match self.func {
                SlidingWindowFunction::Average => T::from(val / (buffer.len() as f64)).ok_or(SquadOvError::BadRequest)?,
                SlidingWindowFunction::PerUnitTime(unit) => {
                    let buffer_time: f64 = (self.buffer_range.end - self.buffer_range.start).num_milliseconds() as f64;
                    let unit_time: f64 = unit.as_millis() as f64;

                    // val / buffer_time gets us the amount per millisecond. Then we multiply by unit_time to get the amount of
                    // the amount value that would happen in unit_time.
                    log::info!("Val: {}, Buffer: {}, Unit: {}", val, buffer_time, unit_time);
                    T::from(val / buffer_time * unit_time).ok_or(SquadOvError::BadRequest)?
                }
            }
        )
    }

    fn compute_next_output_packet_from_buffer(&self) -> Result<OutputAggregatorPacket<T>, SquadOvError> {
        Ok(
            OutputAggregatorPacket{
                start: self.buffer_range.start,
                end: self.buffer_range.end,
                value: if self.buffer.is_empty() {
                    T::from(0i64).ok_or(SquadOvError::BadRequest)?
                } else {
                    self.compute_next_value_from_buffer(&self.buffer)?
                },
            }
        )
    }
}

impl<T> CombatLogAggregator<T> for CombatLogSlidingWindowAggregator<T>
where
    T: num::traits::Zero + std::ops::Div<Output = T> + num::traits::NumCast + num::traits::ToPrimitive + Copy + std::fmt::Debug,
{
    fn handle(&mut self, packet: InputAggregatorPacket<T>) -> Result<Option<OutputAggregatorPacket<T>>, SquadOvError> {
        // If the next packet is outside of the current then we want to flush the values stored in the buffer
        // and bubble that up.
        let ret = if !self.buffer_range.contains(&packet.tm) {
            Some(self.flush()?)
        } else {
            None
        };

        // Now we need to add the packet to the buffer. Note that the 'flush' operation will advance the buffer range for us so we don't need to change that.
        self.buffer.push(packet.data);

        Ok(ret)
    }

    fn flush(&mut self) -> Result<OutputAggregatorPacket<T>, SquadOvError> {
        let packet = self.compute_next_output_packet_from_buffer()?;
        self.clear_buffer();
        Ok(packet)
    }
}