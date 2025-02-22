use std::{
    collections::HashMap,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Default)]
pub struct DiagnosticsCollector {
    bug_reports: Vec<String>,
    scenario_hashes: HashMap<String, String>,
}

impl DiagnosticsCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_bug_report(&mut self, report: &str) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let filename = format!("bug_report_{}.yaml", timestamp);
        fs::write(&filename, report)
            .unwrap_or_else(|_| panic!("Failed to write bug report {}", filename));

        self.bug_reports.push(filename.clone());
        filename
    }

    pub fn record_scenario_hash(&mut self, scenario_path: &Path, hash: &str) {
        self.scenario_hashes.insert(
            scenario_path.to_string_lossy().into_owned(),
            hash.to_string(),
        );
    }
}
