use serde::Serialize;

pub const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Debug, Clone)]
pub struct RequestId(String);

impl RequestId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub request_id: String,
    pub status: u16,
    pub message: String,
    pub success: bool,
    pub data: T,
}

#[derive(Debug, Clone)]
pub struct ResponseMessage(pub String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_ids_are_unique_uuid_v4_values() {
        let first = RequestId::new();
        let second = RequestId::new();
        assert_ne!(first.as_str(), second.as_str());

        for request_id in [first, second] {
            let uuid = uuid::Uuid::parse_str(request_id.as_str()).expect("request id is a UUID");
            assert_eq!(uuid.get_version_num(), 4);
        }
    }
}
