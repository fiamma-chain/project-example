use jsonrpsee::MethodResponse;
use std::{cell::RefCell, mem, sync::Arc};
use thread_local::ThreadLocal;

/// Metadata assigned to a JSON-RPC method call.
#[derive(Debug, Clone)]
pub(crate) struct MethodMetadata {
    pub name: &'static str,
    /// Did this call return an app-level error?
    pub has_app_error: bool,
}

type CurrentMethodInner = RefCell<Option<MethodMetadata>>;

#[must_use = "guard will reset method metadata on drop"]
#[derive(Debug)]
pub(super) struct CurrentMethodGuard<'a> {
    prev: Option<MethodMetadata>,
    current: &'a mut MethodMetadata,
    thread_local: &'a ThreadLocal<CurrentMethodInner>,
}

impl Drop for CurrentMethodGuard<'_> {
    fn drop(&mut self) {
        let cell = self.thread_local.get_or_default();
        *self.current = mem::replace(&mut *cell.borrow_mut(), self.prev.take()).unwrap();
    }
}

/// Tracer of JSON-RPC methods. Can be used to access metadata for the currently handled method call.
// We organize the tracer as a thread-local variable with current method metadata, which is set while the method handler
// is being polled. We use the drop guard pattern to handle corner cases like the handler panicking.
// Method handlers are wrapped using RPC-level middleware in `jsonrpsee`.
#[derive(Debug, Default)]
pub(crate) struct MethodTracer {
    inner: ThreadLocal<CurrentMethodInner>,
}


#[derive(Debug)]
pub(super) struct MethodCall {
    tracer: Arc<MethodTracer>,
    meta: MethodMetadata,
    is_completed: bool,
}

impl MethodCall {
    pub(super) fn set_as_current(&mut self) -> CurrentMethodGuard<'_> {
        let meta = &mut self.meta;
        let cell = self.tracer.inner.get_or_default();
        let prev = mem::replace(&mut *cell.borrow_mut(), Some(meta.clone()));
        CurrentMethodGuard {
            prev,
            current: meta,
            thread_local: &self.tracer.inner,
        }
    }

    pub(super) fn observe_response(&mut self, response: &MethodResponse) {
        self.is_completed = true;
        let meta = &self.meta;
        match response.is_success() {
            true => {
                let msg = format!(
                    "JSON-RPC {} response result {}",
                    meta.name,
                    response.to_result()
                );
                logs::info!(msg);
            }
            false => {
                let msg = format!(
                    "JSON-RPC {} has protocol_error {:?}, has_app_error {}",
                    meta.name,
                    response.as_error_code(),
                    meta.has_app_error
                );
                logs::info!(msg);
            }
        }
    }
}
