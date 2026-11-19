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
pub struct ChecklistItem {
    pub id: u16,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checklist {
    next: u16,
    pub items: Vec<ChecklistItem>,
    done: BTreeMap<String, BTreeSet<u16>>,
}

impl Default for Checklist {
    fn default() -> Self {
        Self {
            next: 1,
            items: Vec::new(),
            done: BTreeMap::new(),
        }
    }
}

impl Checklist {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn add(&mut self, title: &str) -> Result<(), &'static str> {
        let title = label(title);
        if title.is_empty() {
            return Err("Enter a checklist item.");
        }
        if self.items.len() >= 24 || self.next == u16::MAX {
            return Err("Checklist item limit reached.");
        }
        self.items.push(ChecklistItem {
            id: self.next,
            title,
        });
        self.next += 1;
        Ok(())
    }

    pub fn remove(&mut self, id: u16) {
        self.items.retain(|item| item.id != id);
        self.prune();
    }

    pub fn checked(&self, day: Day, id: u16) -> bool {
        self.done
            .get(&day.to_string())
            .is_some_and(|items| items.contains(&id))
    }

    fn set_checked(&mut self, day: Day, id: u16, done: bool) -> Result<(), &'static str> {
        if !self.items.iter().any(|item| item.id == id) {
            return Err("Checklist item no longer exists.");
        }
        let key = day.to_string();
        if done {
            if self.done.len() >= 4096 && !self.done.contains_key(&key) {
                return Err("Checklist history limit reached.");
            }
            self.done.entry(key).or_default().insert(id);
        } else if let Some(items) = self.done.get_mut(&key) {
            items.remove(&id);
            if items.is_empty() {
                self.done.remove(&key);
            }
        }
        Ok(())
    }

    fn prune(&mut self) {
        let ids: BTreeSet<_> = self.items.iter().map(|item| item.id).collect();
        self.done.retain(|_, items| {
            items.retain(|id| ids.contains(id));
            !items.is_empty()
        });
    }

    fn valid(&self) -> bool {
        let mut ids = BTreeSet::new();
        self.next > 0
            && self.items.len() <= 24
            && self.done.len() <= 4096
            && self.items.iter().all(|item| {
                item.id > 0
                    && item.id < self.next
                    && ids.insert(item.id)
                    && !label(&item.title).is_empty()
                    && item.title.len() <= 480
            })
            && self.done.iter().all(|(day, items)| {
                day.parse::<Day>().is_ok() && !items.is_empty() && items.is_subset(&ids)
            })
    }
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
    #[serde(default, skip_serializing_if = "Checklist::is_default")]
    pub checklist: Checklist,
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
            checklist: Checklist::default(),
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

    pub fn set_checklist_item(
        &mut self,
        day: Day,
        id: u16,
        checked: bool,
    ) -> Result<(), &'static str> {
        if !self.occurs(day) {
            return Err("The task is not scheduled on this date.");
        }
        self.checklist.set_checked(day, id, checked)
    }

    pub fn duplicate_for(&self, day: Day, now: i64) -> Self {
        let mut copy = self.clone();
        copy.id = 0;
        copy.day = day;
        copy.created = now;
        copy.completions.clear();
        copy.skipped.clear();
        copy.checklist.done.clear();
        copy
    }

    pub fn overdue(&self, today: Day) -> bool {
        self.repeat == Repeat::Once
            && self.day < today
            && !self.completions.contains_key(&self.day.to_string())
    }

    fn valid(&self) -> bool {
        self.day.valid()
            && self.checklist.valid()
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomExamDate {
    pub exam: Exam,
    pub day: Day,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Planner {
    next: Id,
    pub tasks: Vec<Task>,
    pub topics: Vec<Topic>,
    pub profile: Option<Profile>,
    pub weekly_goal_minutes: Option<u16>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub custom_exam_dates: Vec<CustomExamDate>,
}

impl Default for Planner {
    fn default() -> Self {
        Self {
            next: 1,
            tasks: vec![],
            topics: vec![],
            profile: None,
            weekly_goal_minutes: None,
            custom_exam_dates: vec![],
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
            task.checklist.done = std::mem::take(&mut previous.checklist.done);
            task.checklist.next = task.checklist.next.max(previous.checklist.next);
            task.checklist.prune();
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

    pub fn has_new_fields(&self) -> bool {
        !self.custom_exam_dates.is_empty()
            || self.tasks.iter().any(|task| !task.checklist.is_default())
    }

    pub fn set_custom_exam_date(
        &mut self,
        exam: Exam,
        year: u16,
        day: Option<Day>,
    ) -> Result<(), &'static str> {
        if day.is_some_and(|day| !day.valid() || day.year != year || year < 2020) {
            return Err("Choose a date within the selected exam year.");
        }
        let index = self
            .custom_exam_dates
            .iter()
            .position(|record| record.exam == exam && record.day.year == year);
        match (index, day) {
            (Some(index), Some(day)) => self.custom_exam_dates[index].day = day,
            (Some(index), None) => {
                self.custom_exam_dates.remove(index);
            }
            (None, Some(day)) if self.custom_exam_dates.len() < 256 => {
                self.custom_exam_dates.push(CustomExamDate { exam, day })
            }
            (None, Some(_)) => return Err("Custom exam date limit reached."),
            (None, None) => {}
        }
        Ok(())
    }

    pub fn custom_exams_on(&self, day: Day) -> impl Iterator<Item = Exam> + '_ {
        let active = self.profile.filter(|profile| profile.year == day.year);
        self.custom_exam_dates
            .iter()
            .filter(move |record| {
                record.day == day
                    && active.is_some_and(|profile| profile.track.exams().contains(&record.exam))
            })
            .map(|record| record.exam)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let mut dates = BTreeSet::new();
        if self.custom_exam_dates.len() > 256
            || self.custom_exam_dates.iter().any(|record| {
                !record.day.valid()
                    || record.day.year < 2020
                    || !dates.insert((record.exam as u8, record.day.year))
            })
        {
            return Err("Invalid custom exam date.");
        }
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
    fn checklist_completion_is_per_occurrence_and_stable_across_edits() {
        let first: Day = "2027-01-07".parse().unwrap();
        let later: Day = "2027-01-08".parse().unwrap();
        let mut planner = Planner::default();
        let mut task = Task::new(first, "Review", 1);
        task.repeat = Repeat::Daily;
        task.checklist.add("Read").unwrap();
        task.checklist.add("Practice").unwrap();
        let id = planner.save_task(task).unwrap();
        let item = planner.tasks[0].checklist.items[0].id;
        planner.tasks[0]
            .set_checklist_item(first, item, true)
            .unwrap();
        let mut draft = planner.tasks[0].clone();
        draft.checklist.add("Check errors").unwrap();
        draft.checklist.items[0].title = "Read the worked example".into();
        assert_eq!(planner.save_task(draft).unwrap(), id);
        assert!(planner.tasks[0].checklist.checked(first, item));
        assert!(!planner.tasks[0].checklist.checked(later, item));
        assert!(planner.tasks[0].completions.is_empty());
        let duplicate = planner.tasks[0].duplicate_for(first, 10);
        assert!(!duplicate.checklist.checked(first, item));
        planner.tasks[0].skip(later);
        assert!(planner.tasks[0]
            .set_checklist_item(later, item, true)
            .is_err());
        planner.tasks[0].checklist.remove(item);
        assert!(planner.validate().is_ok());
    }

    #[test]
    fn custom_dates_are_filtered_by_active_exam_and_year() {
        let date: Day = "2027-04-07".parse().unwrap();
        let mut plan = Planner {
            profile: Some(Profile {
                track: Track::Main,
                year: 2027,
            }),
            ..Planner::default()
        };
        plan.set_custom_exam_date(Exam::Main, 2027, Some(date))
            .unwrap();
        plan.set_custom_exam_date(Exam::Advanced, 2027, Some(date))
            .unwrap();
        assert!(plan
            .set_custom_exam_date(Exam::Neet, 2026, Some(date))
            .is_err());
        assert_eq!(plan.custom_exams_on(date).collect::<Vec<_>>(), [Exam::Main]);
        plan.profile = Some(Profile {
            track: Track::Both,
            year: 2027,
        });
        assert_eq!(plan.custom_exams_on(date).count(), 2);
        plan.set_custom_exam_date(Exam::Main, 2027, None).unwrap();
        assert_eq!(
            plan.custom_exams_on(date).collect::<Vec<_>>(),
            [Exam::Advanced]
        );
    }

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
