//! Hyper service that adds a context to an incoming request and passes it on
//! to a wrapped service.

use crate::{ContextualPayload, Push, XSpanId};
use std::marker::PhantomData;
use std::task::{Context, Poll};

/// Middleware wrapper service, that should be used as the outermost layer in a
/// stack of hyper services. Adds a context to a plain `hyper::Request` that can be
/// used by subsequent layers in the stack.
#[derive(Debug)]
pub struct AddContextMakeService<C> {
    phantom: PhantomData<C>
}

impl<C> AddContextMakeService<C> {
    /// Create a new AddContextMakeService struct wrapping a value
    pub fn new() -> Self {
        AddContextMakeService {
            phantom: PhantomData,
        }
    }
}

impl<T, C> hyper::service::Service<T> for AddContextMakeService<C> {
    type Response = AddContextService<T, C>;
    type Error = std::io::Error;
    type Future = futures::future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, inner: T) -> Self::Future {
        futures::future::ok(AddContextService::new(inner))
    }
}

/// Middleware wrapper service, that should be used as the outermost layer in a
/// stack of hyper services. Adds a context to a plain `hyper::Request` that can be
/// used by subsequent layers in the stack. The `AddContextService` struct should
/// not usually be used directly - when constructing a hyper stack use
/// `AddContextMakeService`, which will create `AddContextService` instances as needed.
#[derive(Debug)]
pub struct AddContextService<T, C> {
    inner: T,
    marker: PhantomData<C>,
}

impl<T, C> AddContextService<T, C> {
    /// Create a new AddContextService struct wrapping a value
    pub fn new(inner: T) -> Self {
        AddContextService {
            inner,
            marker: PhantomData,
        }
    }
}

impl<T, C> hyper::service::Service<hyper::Request<hyper::Body>> for AddContextService<T, C>
    where
        C: Default + Push<XSpanId> + Send + Sync + 'static,
        C::Result: Send + Sync + 'static,
        T: hyper::service::Service<ContextualPayload<C::Result>>,
{
    type Response = T::Response;
    type Error = T::Error;
    type Future = T::Future;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: hyper::Request<hyper::Body>) -> Self::Future {
        let x_span_id = XSpanId::get_or_generate(&req);
        let context = C::default().push(x_span_id);

        self.inner.call(ContextualPayload {
            inner: req,
            context: context,
        })
    }
}


