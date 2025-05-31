use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Job {
    pub name: String,
    pub args: Vec<String>,
    pub envs: HashMap<String, String>,
}
