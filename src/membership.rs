use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct NodeInfo {
    pub id: String,
    pub address: String,
    pub status: String,
}

pub fn default_membership() -> Vec<NodeInfo> {
    vec![
        NodeInfo {
            id: "node-a".to_string(),
            address: "http://127.0.0.1:7100".to_string(),
            status: "healthy".to_string(),
        },
        NodeInfo {
            id: "node-b".to_string(),
            address: "http://127.0.0.1:7101".to_string(),
            status: "healthy".to_string(),
        },
        NodeInfo {
            id: "node-c".to_string(),
            address: "http://127.0.0.1:7102".to_string(),
            status: "healthy".to_string(),
        },
    ]
}
