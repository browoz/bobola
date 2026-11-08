use crate::Result;
use adblock::{
    engine::Engine,
    lists::{FilterSet, ParseOptions},
    request::Request,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

// This is our small original starter list, not a copy of EasyList.
pub const STARTER_FILTERS: &str = "[Adblock Plus 2.0]\n! Service Workspace starter rules\n||doubleclick.net^\n||googlesyndication.com^\n||googleadservices.com^\n";

pub struct Protection {
    engine: Engine,
    pub revision: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub blocked: bool,
    pub exception: bool,
}

impl Protection {
    pub fn compile(text: &str) -> Result<Self> {
        if text.len() > 12 * 1024 * 1024
            || text.trim().is_empty()
            || text.trim_start().starts_with('<')
        {
            return Err(
                "The filter list is empty, too large, or contains an HTML response.".into(),
            );
        }
        let rules: Vec<&str> = text
            .lines()
            .filter(|line| {
                !line.trim().is_empty() && !line.starts_with('!') && !line.starts_with('[')
            })
            .collect();
        if rules.is_empty() {
            return Err("The update contains no filter rules.".into());
        }
        let mut filters = FilterSet::new(false);
        filters.add_filter_list(text.to_owned(), ParseOptions::default());
        let engine = Engine::new_with_filter_set(filters);
        let revision = format!("{:x}", Sha256::digest(text.as_bytes()));
        Ok(Self { engine, revision })
    }

    pub fn check(&self, url: &str, source: &str, kind: &str, enabled: bool) -> Decision {
        self.check_method(url, source, kind, "get", enabled)
    }

    pub fn check_method(
        &self,
        url: &str,
        source: &str,
        kind: &str,
        method: &str,
        enabled: bool,
    ) -> Decision {
        if !enabled {
            return Decision {
                blocked: false,
                exception: false,
            };
        }
        let Ok(request) = Request::new(url, source, kind, method) else {
            return Decision {
                blocked: false,
                exception: false,
            };
        };
        let result = self.engine.check_network_request(&request);
        Decision {
            blocked: result.should_block(),
            exception: result.exception.is_some(),
        }
    }

    pub fn cosmetic_css(&self, url: &str, classes: &[String], ids: &[String]) -> String {
        let resources = self.engine.url_cosmetic_resources(url);
        let mut selectors: Vec<String> = resources.hide_selectors.into_iter().collect();
        if !resources.generichide {
            selectors.extend(self.engine.hidden_class_id_selectors(
                classes,
                ids,
                &resources.exceptions,
            ));
        }
        selectors.sort();
        selectors.dedup();
        selectors
            .into_iter()
            .filter(|selector| !selector.is_empty())
            .map(|selector| format!("{selector}{{display:none!important}}\n"))
            .collect()
    }
}
