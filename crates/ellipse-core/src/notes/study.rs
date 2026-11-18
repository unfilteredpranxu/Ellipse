use super::{label, Id};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interval {
    pub start: i64,
    pub end: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionKind {
    #[default]
    Focus,
    Break,
    Stopwatch,
}

impl SessionKind {
    fn is_focus(&self) -> bool {
        *self == Self::Focus
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Focus => "Focus",
            Self::Break => "Break",
            Self::Stopwatch => "Stopwatch",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub subject: String,
    pub book: Option<Id>,
    pub target_ms: Option<i64>,
    pub started: i64,
    pub running_since: Option<i64>,
    pub intervals: Vec<Interval>,
    pub finished: Option<i64>,
    #[serde(default, skip_serializing_if = "SessionKind::is_focus")]
    pub kind: SessionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<Id>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Study {
    pub active: Option<Session>,
    pub history: Vec<Session>,
}

impl Session {
    pub fn elapsed(&self, now: i64) -> i64 {
        let recorded: i64 = self
            .intervals
            .iter()
            .map(|i| i.end.saturating_sub(i.start).max(0))
            .fold(0, i64::saturating_add);
        let elapsed = recorded.saturating_add(
            self.running_since
                .map_or(0, |since| now.saturating_sub(since).max(0)),
        );
        self.target_ms.map_or(elapsed, |target| elapsed.min(target))
    }

    pub fn pause(&mut self, now: i64) {
        if let Some(start) = self.running_since.take() {
            let elapsed = self.elapsed(start);
            let available = self.target_ms.map_or(i64::MAX, |t| (t - elapsed).max(0));
            let end = now.max(start).min(start.saturating_add(available));
            if end > start {
                self.intervals.push(Interval { start, end });
            }
        }
    }

    pub fn resume(&mut self, now: i64) {
        if self.running_since.is_none()
            && self.finished.is_none()
            && self
                .target_ms
                .is_none_or(|target| self.elapsed(now) < target)
        {
            self.running_since =
                Some(now.max(self.intervals.last().map_or(self.started, |i| i.end)));
        }
    }

    pub fn milliseconds_in(&self, from: i64, to: i64) -> i64 {
        self.intervals
            .iter()
            .map(|i| i.end.min(to).saturating_sub(i.start.max(from)).max(0))
            .fold(0, i64::saturating_add)
    }
}

impl Study {
    pub(super) fn has_extended_fields(&self) -> bool {
        self.history.iter().chain(&self.active).any(|session| {
            !session.kind.is_focus() || session.task.is_some() || session.topic.is_some()
        })
    }

    pub fn start(
        &mut self,
        subject: &str,
        book: Option<Id>,
        minutes: Option<u16>,
        now: i64,
    ) -> bool {
        if self.active.is_some() || self.history.len() >= 20000 {
            return false;
        }
        let now = self
            .history
            .iter()
            .filter_map(|session| session.finished)
            .max()
            .map_or(now.max(0), |previous| now.max(previous));
        self.active = Some(Session {
            subject: label(subject),
            book,
            target_ms: minutes.map(|m| i64::from(m.clamp(1, 540)) * 60000),
            started: now,
            running_since: Some(now),
            intervals: vec![],
            finished: None,
            kind: SessionKind::Focus,
            task: None,
            topic: None,
        });
        true
    }

    pub fn finish(&mut self, now: i64) -> bool {
        let Some(mut active) = self.active.take() else {
            return false;
        };
        active.pause(now);
        active.finished = Some(
            now.max(active.started)
                .max(active.intervals.last().map_or(0, |i| i.end)),
        );
        self.history.push(active);
        true
    }

    pub(super) fn validate(&self) -> Result<(), &'static str> {
        if self.history.len() > 20000
            || self.history.iter().chain(&self.active).any(|session| {
                session.intervals.len() > 4096
                    || session.started < 0
                    || session.subject.len() > 480
                    || session
                        .target_ms
                        .is_some_and(|target| !(60_000..=32_400_000).contains(&target))
                    || session
                        .intervals
                        .iter()
                        .any(|i| i.start < session.started || i.start >= i.end)
                    || session.intervals.windows(2).any(|i| i[0].end > i[1].start)
            })
        {
            return Err("Invalid study history.");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paused_segments_split_at_midnight_and_clock_rollback_cannot_overlap_finished_session() {
        let midnight = 86_400_000;
        let mut study = Study::default();
        assert!(study.start("Physics", None, None, midnight - 60_000));
        study.active.as_mut().unwrap().pause(midnight + 30_000);
        study.active.as_mut().unwrap().resume(midnight + 90_000);
        assert!(study.finish(midnight + 150_000));
        assert_eq!(study.history[0].milliseconds_in(0, midnight), 60_000);
        assert_eq!(
            study.history[0].milliseconds_in(midnight, midnight + 200_000),
            90_000
        );
        assert!(study.start("", None, Some(5), midnight - 120_000));
        assert_eq!(study.active.as_ref().unwrap().started, midnight + 150_000);
        assert_eq!(study.active.as_ref().unwrap().elapsed(midnight - 1), 0);
    }

    #[test]
    fn suspension_pause_and_day_boundaries() {
        let mut study = Study::default();
        assert!(study.start("Math", None, Some(5), 1000));
        assert!(!study.start("Double count", None, None, 1001));
        let session = study.active.as_mut().unwrap();
        session.pause(81_000);
        assert_eq!(session.elapsed(151_000), 80_000);
        session.resume(201_000);
        assert_eq!(session.elapsed(801_000), 300_000);
        assert!(study.finish(801_000));
        let session = &study.history[0];
        assert_eq!(session.milliseconds_in(0, 250_000), 129_000);
        assert_eq!(session.milliseconds_in(250_000, i64::MAX), 171_000);
    }
}
