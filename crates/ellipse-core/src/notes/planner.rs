//! Calendar tasks and personal exam progress share the vault's checkpoint.
use super::{label, Id};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    str::FromStr,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Day {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl Day {
    pub fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        let date = Self { year, month, day };
        ((1900..=2200).contains(&year)
            && (1..=12).contains(&month)
            && (1..=date.month_days()).contains(&day))
        .then_some(date)
    }

    pub fn valid(self) -> bool {
        Self::new(self.year, self.month, self.day).is_some()
    }

    pub fn month_days(self) -> u8 {
        match self.month {
            2 if self.year.is_multiple_of(4)
                && (!self.year.is_multiple_of(100) || self.year.is_multiple_of(400)) =>
            {
                29
            }
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        }
    }

    pub fn ordinal(self) -> i32 {
        let y = i32::from(self.year) - 1;
        let prior = 365 * y + y / 4 - y / 100 + y / 400;
        prior
            + (1..self.month)
                .map(|month| i32::from(Self { month, ..self }.month_days()))
                .sum::<i32>()
            + i32::from(self.day)
    }

    /// Monday = 0.
    pub fn weekday(self) -> u8 {
        ((self.ordinal() - 1).rem_euclid(7)) as u8
    }
}

impl fmt::Display for Day {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl FromStr for Day {
    type Err = ();
    fn from_str(text: &str) -> Result<Self, ()> {
        let mut parts = text.split('-');
        let day = Self::new(
            parts.next().ok_or(())?.parse().map_err(|_| ())?,
            parts.next().ok_or(())?.parse().map_err(|_| ())?,
            parts.next().ok_or(())?.parse().map_err(|_| ())?,
        )
        .ok_or(())?;
        (parts.next().is_none() && text == day.to_string())
            .then_some(day)
            .ok_or(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Repeat {
    #[default]
    Once,
    Daily,
    Weekly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: Id,
    pub title: String,
    pub details: String,
    pub subject: String,
    pub day: Day,
    /// Minutes from midnight on the task's civil date.
    pub time: Option<u16>,
    pub estimate: Option<u16>,
    pub priority: Priority,
    pub repeat: Repeat,
    pub book: Option<Id>,
    pub topic: Option<Id>,
    pub created: i64,
    pub completions: BTreeMap<String, i64>,
    pub skipped: BTreeSet<String>,
}

impl Task {
    pub fn new(day: Day, title: &str, now: i64) -> Self {
        Self {
            id: 0,
            title: label(title),
            details: String::new(),
            subject: String::new(),
            day,
            time: None,
            estimate: None,
            priority: Priority::Normal,
            repeat: Repeat::Once,
            book: None,
            topic: None,
            created: now,
            completions: BTreeMap::new(),
            skipped: BTreeSet::new(),
        }
    }

    pub fn scheduled(&self, day: Day) -> bool {
        let delta = day.ordinal() - self.day.ordinal();
        delta >= 0
            && match self.repeat {
                Repeat::Once => delta == 0,
                Repeat::Daily => true,
                Repeat::Weekly => delta % 7 == 0,
            }
    }

    pub fn occurs(&self, day: Day) -> bool {
        self.scheduled(day) && !self.skipped.contains(&day.to_string())
    }

    pub fn set_done(&mut self, day: Day, done: bool, now: i64) {
        if !self.occurs(day) {
            return;
        }
        let key = day.to_string();
        if done && now >= 0 && self.completions.len() < 4096 {
            self.completions.entry(key).or_insert(now);
        } else if !done {
            self.completions.remove(&key);
        }
    }

    pub fn skip(&mut self, day: Day) {
        if self.repeat != Repeat::Once && self.occurs(day) && self.skipped.len() < 4096 {
            self.skipped.insert(day.to_string());
        }
    }

    fn valid(&self) -> bool {
        self.day.valid()
            && !label(&self.title).is_empty()
            && self.title.len() <= 480
            && self.details.len() <= 4000
            && self.subject.len() <= 480
            && self.created >= 0
            && self.time.is_none_or(|time| time < 1440)
            && self
                .estimate
                .is_none_or(|estimate| (1..=1440).contains(&estimate))
            && self.completions.len() <= 4096
            && self.skipped.len() <= 4096
            && self
                .completions
                .iter()
                .all(|(day, time)| day.parse::<Day>().is_ok() && *time >= 0)
            && self.skipped.iter().all(|day| day.parse::<Day>().is_ok())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Track {
    Main,
    Both,
    Neet,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Exam {
    Main,
    Advanced,
    Neet,
}

impl Track {
    pub fn exams(self) -> &'static [Exam] {
        match self {
            Self::Main => &[Exam::Main],
            Self::Both => &[Exam::Main, Exam::Advanced],
            Self::Neet => &[Exam::Neet],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub track: Track,
    pub year: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Subject {
    Physics,
    Chemistry,
    Mathematics,
    Biology,
}

impl Exam {
    pub fn label(self) -> &'static str {
        match self {
            Self::Main => "JEE Main",
            Self::Advanced => "JEE Advanced",
            Self::Neet => "NEET UG",
        }
    }
    pub fn subjects(self) -> [Subject; 3] {
        [
            Subject::Physics,
            Subject::Chemistry,
            if self == Self::Neet {
                Subject::Biology
            } else {
                Subject::Mathematics
            },
        ]
    }
}

impl Subject {
    pub fn label(self) -> &'static str {
        match self {
            Self::Physics => "Physics",
            Self::Chemistry => "Chemistry",
            Self::Mathematics => "Mathematics",
            Self::Biology => "Biology",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Progress {
    #[default]
    NotStarted,
    Learning,
    Practicing,
    Revised,
    Completed,
}

impl Progress {
    pub const ALL: [Self; 5] = [
        Self::NotStarted,
        Self::Learning,
        Self::Practicing,
        Self::Revised,
        Self::Completed,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::NotStarted => "Not started",
            Self::Learning => "Learning",
            Self::Practicing => "Practicing",
            Self::Revised => "Revised",
            Self::Completed => "Completed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Topic {
    pub id: Id,
    pub exam: Exam,
    pub subject: Subject,
    pub unit: String,
    pub title: String,
    pub progress: Progress,
    pub weak: bool,
    pub changed: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Planner {
    next: Id,
    pub tasks: Vec<Task>,
    pub topics: Vec<Topic>,
    pub profile: Option<Profile>,
    pub weekly_goal_minutes: Option<u16>,
}

impl Default for Planner {
    fn default() -> Self {
        Self {
            next: 1,
            tasks: vec![],
            topics: vec![],
            profile: None,
            weekly_goal_minutes: None,
        }
    }
}

impl Planner {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn save_task(&mut self, mut task: Task) -> Result<Id, &'static str> {
        if !task.valid() {
            return Err("Enter a task title and valid date and time.");
        }
        if task.id == 0 {
            if self.tasks.len() >= 4096 || self.next == u32::MAX {
                return Err("Calendar task limit reached.");
            }
            task.id = self.next;
            self.next += 1;
            let id = task.id;
            self.tasks.push(task);
            Ok(id)
        } else {
            let previous = self
                .tasks
                .iter_mut()
                .find(|item| item.id == task.id)
                .ok_or("The task no longer exists.")?;
            task.completions = std::mem::take(&mut previous.completions);
            task.skipped = std::mem::take(&mut previous.skipped);
            let id = task.id;
            *previous = task;
            Ok(id)
        }
    }

    pub fn add_topic(&mut self, mut topic: Topic) -> Result<(), &'static str> {
        if !topic.valid() {
            return Err("Choose a subject and enter a unit and topic.");
        }
        if self.topics.len() >= 4096 || self.next == u32::MAX {
            return Err("Personal topic limit reached.");
        }
        topic.id = self.next;
        self.next += 1;
        self.topics.push(topic);
        Ok(())
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let mut ids = BTreeSet::new();
        let mut id = |id| id > 0 && id < self.next && ids.insert(id);
        if self
            .weekly_goal_minutes
            .is_some_and(|goal| !(1..=10080).contains(&goal))
            || self.next == 0
            || self.tasks.len() > 4096
            || self.topics.len() > 4096
            || self
                .profile
                .is_some_and(|profile| !(2020..=2200).contains(&profile.year))
            || self.tasks.iter().any(|task| !task.valid() || !id(task.id))
            || self
                .topics
                .iter()
                .any(|topic| !topic.valid() || !id(topic.id))
        {
            return Err("Invalid Calendar or Study records.");
        }
        Ok(())
    }
}

impl Topic {
    fn valid(&self) -> bool {
        self.exam.subjects().contains(&self.subject)
            && self.unit.len() <= 480
            && self.title.len() <= 480
            && !label(&self.unit).is_empty()
            && !label(&self.title).is_empty()
            && self.changed >= 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_recurrence_survives_leap_day_and_has_independent_occurrences() {
        let first: Day = "2024-02-29".parse().unwrap();
        let week_later: Day = "2024-03-07".parse().unwrap();
        assert_eq!(first.weekday(), 3);
        assert!("2025-02-29".parse::<Day>().is_err());
        let mut task = Task::new(first, "Review mechanics", 15);
        task.repeat = Repeat::Weekly;
        task.set_done(first, true, 20);
        task.set_done(first, true, 25);
        task.skip(week_later);
        assert!(task.occurs(first));
        assert_eq!(task.completions[&first.to_string()], 20);
        assert!(!task.occurs(week_later));
        task.set_done(week_later, true, 33);
        assert_eq!(task.completions.len(), 1);
    }

    #[test]
    fn switching_tracks_preserves_topics_and_linked_tasks() {
        let mut planner = Planner {
            profile: Some(Profile {
                track: Track::Both,
                year: 2027,
            }),
            ..Planner::default()
        };
        planner
            .add_topic(Topic {
                id: 0,
                exam: Exam::Advanced,
                subject: Subject::Mathematics,
                unit: "Personal unit".into(),
                title: "Personal topic".into(),
                progress: Progress::Revised,
                weak: true,
                changed: 10,
            })
            .unwrap();
        let mut task = Task::new("2027-01-01".parse().unwrap(), "Review", 12);
        task.topic = Some(planner.topics[0].id);
        planner.save_task(task).unwrap();
        planner.profile = Some(Profile {
            track: Track::Neet,
            year: 2028,
        });
        assert_eq!(planner.topics[0].progress, Progress::Revised);
        assert_eq!(planner.tasks[0].topic, Some(planner.topics[0].id));
        assert!(planner.validate().is_ok());
    }
}
