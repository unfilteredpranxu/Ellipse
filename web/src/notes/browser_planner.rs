use ellipse_core::notes::{Study, Vault, planner::Planner};
use leptos::prelude::window;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct BrowserData {
    version: u32,
    planner: Planner,
    study: Study,
}

pub(super) struct BrowserPlanner {
    key: String,
    previous: Option<String>,
    blocked: bool,
}

impl BrowserPlanner {
    pub(super) fn open(uid: &str, vault: &mut Vault) -> (Self, Option<String>) {
        let key = format!("ellipse:planner:v1:{uid}");
        let mut store = Self {
            key,
            previous: None,
            blocked: false,
        };
        let result = store.load(vault);
        if result.is_err() {
            store.blocked = true;
        }
        (store, result.err())
    }

    fn load(&mut self, vault: &mut Vault) -> Result<(), String> {
        let storage = window()
            .local_storage()
            .ok()
            .flatten()
            .ok_or("Browser storage is unavailable.")?;
        self.previous = storage
            .get_item(&self.key)
            .map_err(|_| "Browser storage could not be read.")?;
        if let Some(raw) = &self.previous {
            if raw.len() > 2_000_000 {
                return Err("The saved browser planner exceeds its size limit.".into());
            }
            let data: BrowserData = serde_json::from_str(raw)
                .map_err(|_| "The saved browser planner could not be read.")?;
            if !(1..=2).contains(&data.version) {
                return Err(
                    "The saved browser planner requires a different version of Ellipse.".into(),
                );
            }
            let candidate = Vault {
                planner: data.planner,
                study: data.study,
                ..Vault::default()
            };
            candidate.validate()?;
            vault.planner = candidate.planner;
            vault.study = candidate.study;
        }
        Ok(())
    }

    pub(super) fn save(&mut self, vault: &Vault) -> Result<(), String> {
        if self.blocked {
            return Err(
                "The browser planner could not be opened. Its stored data was preserved.".into(),
            );
        }
        let storage = window()
            .local_storage()
            .ok()
            .flatten()
            .ok_or("Browser storage is unavailable.")?;
        let current = storage
            .get_item(&self.key)
            .map_err(|_| "Browser storage could not be read.")?;
        if current != self.previous {
            return Err(
                "The browser planner changed in another tab. Reopen Calendar before editing."
                    .into(),
            );
        }
        let candidate = Vault {
            planner: vault.planner.clone(),
            study: vault.study.clone(),
            ..Vault::default()
        };
        candidate.validate()?;
        let raw = serde_json::to_string(&BrowserData {
            version: 2,
            planner: candidate.planner,
            study: candidate.study,
        })
        .map_err(|_| "The browser planner could not be encoded.")?;
        if raw.len() > 2_000_000 {
            return Err("The browser planner is full.".into());
        }
        storage
            .set_item(&self.key, &raw)
            .map_err(|_| "Browser storage could not save this change.")?;
        self.previous = Some(raw);
        Ok(())
    }
}
