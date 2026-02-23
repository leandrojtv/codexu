use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: usize,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub goal: String,
    pub steps: Vec<PlanStep>,
}

#[derive(Default)]
pub struct Planner;

impl Planner {
    pub fn create_plan(&self, goal: &str) -> Plan {
        Plan {
            goal: goal.to_string(),
            steps: vec![
                PlanStep {
                    id: 1,
                    title: "Entender tarefa".to_string(),
                    status: "pending".to_string(),
                },
                PlanStep {
                    id: 2,
                    title: "Propor alteração em diff".to_string(),
                    status: "pending".to_string(),
                },
                PlanStep {
                    id: 3,
                    title: "Validar e reportar".to_string(),
                    status: "pending".to_string(),
                },
            ],
        }
    }
}
