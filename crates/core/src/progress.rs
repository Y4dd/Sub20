//! Reading FFmpeg's `-progress` stream.
//!
//! FFmpeg writes blocks of `key=value` lines, each block ending with
//! `progress=continue` or `progress=end`.

/// One finished block.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tick {
    /// Seconds of output written so far.
    pub out_time: f64,
    /// Bytes written so far.
    pub total_size: u64,
    /// Encoding speed relative to real time.
    pub speed: Option<f64>,
    pub ended: bool,
}

/// Collects lines into [`Tick`]s.
#[derive(Default)]
pub struct Reader {
    current: Tick,
}

impl Reader {
    /// Feeds one line. Returns a tick when the line closes a block.
    pub fn line(&mut self, line: &str) -> Option<Tick> {
        let (key, value) = line.trim().split_once('=')?;
        let value = value.trim();
        match key {
            // Both carry microseconds, despite the name of the second one.
            "out_time_us" | "out_time_ms" => {
                if let Ok(us) = value.parse::<i64>() {
                    if us >= 0 {
                        self.current.out_time = us as f64 / 1_000_000.0;
                    }
                }
            }
            "total_size" => {
                if let Ok(bytes) = value.parse::<u64>() {
                    self.current.total_size = bytes;
                }
            }
            "speed" => {
                self.current.speed =
                    value.trim_end_matches('x').trim().parse::<f64>().ok().filter(|s| s.is_finite() && *s > 0.0);
            }
            "progress" => {
                self.current.ended = value == "end";
                return Some(self.current);
            }
            _ => {}
        }
        None
    }
}

/// Smooths the speed so the time estimate does not jump around.
#[derive(Default)]
pub struct Eta {
    smoothed: Option<f64>,
}

impl Eta {
    /// Seconds of work left, given how much output remains.
    pub fn update(&mut self, speed: Option<f64>, remaining_output_seconds: f64) -> Option<f64> {
        if let Some(speed) = speed {
            self.smoothed = Some(match self.smoothed {
                Some(previous) => previous * 0.7 + speed * 0.3,
                None => speed,
            });
        }
        let speed = self.smoothed?;
        (speed > 0.0).then(|| (remaining_output_seconds / speed).max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_blocks() {
        let mut reader = Reader::default();
        let block = "frame=120\nfps=60.1\nbitrate= 950.2kbits/s\ntotal_size=594000\nout_time_us=5000000\nout_time=00:00:05.000000\nspeed=2.5x\nprogress=continue";
        let ticks: Vec<_> = block.lines().filter_map(|l| reader.line(l)).collect();
        assert_eq!(ticks.len(), 1);
        assert_eq!(ticks[0], Tick { out_time: 5.0, total_size: 594000, speed: Some(2.5), ended: false });

        assert!(reader.line("speed=N/A").is_none());
        let end = reader.line("progress=end").unwrap();
        assert!(end.ended);
        assert_eq!(end.speed, None);
    }

    #[test]
    fn ignores_garbage_and_negative_times() {
        let mut reader = Reader::default();
        assert!(reader.line("no equals sign").is_none());
        reader.line("out_time_us=-9223372036854775807");
        reader.line("total_size=N/A");
        assert_eq!(reader.line("progress=continue").unwrap().out_time, 0.0);
    }

    #[test]
    fn eta_smooths() {
        let mut eta = Eta::default();
        assert_eq!(eta.update(None, 10.0), None);
        assert_eq!(eta.update(Some(2.0), 10.0), Some(5.0));
        let next = eta.update(Some(4.0), 10.0).unwrap();
        assert!(next > 3.0 && next < 5.0);
    }
}
