use axum::body::Body;
use std::sync::{Arc, Mutex};

use crate::api_type::ApiTypeHandler;
use crate::request_metadata::RequestInspectionMetadata;

use super::CallbackInspector;
use super::stream::InspectorStream;

#[derive(Debug)]
pub struct RequestInspector {
    slot: Option<Arc<Mutex<Option<RequestInspectionMetadata>>>>,
}

impl RequestInspector {
    /// If `api_type_handler` is Some, wraps the request body with an inspector stream.
    /// If None, returns the request unchanged and `take()` will return None.
    pub fn new(
        api_type_handler: Option<&(dyn ApiTypeHandler + Send)>,
        request: axum::extract::Request,
    ) -> (Self, axum::extract::Request) {
        let Some(handler) = api_type_handler else {
            return (Self { slot: None }, request);
        };

        let slot: Arc<Mutex<Option<RequestInspectionMetadata>>> = Arc::new(Mutex::new(None));
        let slot_clone = Arc::clone(&slot);
        let inspector = handler.request_inspector();

        let inspector = CallbackInspector::new(inspector, move |result| {
            if let Ok(metadata) = result
                && let Ok(mut guard) = slot_clone.lock()
            {
                *guard = Some(metadata.clone());
            }
        });

        let (parts, body) = request.into_parts();
        let stream = InspectorStream::new(body.into_data_stream(), Box::new(inspector));
        let request = axum::http::Request::from_parts(parts, Body::from_stream(stream));

        (Self { slot: Some(slot) }, request)
    }

    /// Consume the inspector and return the inspection result.
    pub fn take(self) -> Option<RequestInspectionMetadata> {
        let slot = self.slot?;
        match slot.lock() {
            Ok(mut guard) => guard.take(),
            Err(e) => {
                tracing::error!("Failed to lock request inspection metadata: {e}");
                None
            }
        }
    }
}
