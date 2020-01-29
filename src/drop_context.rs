//! Hyper service that adds a context to an incoming request and passes it on
//! to a wrapped service.

use crate::ContextualPayload;
use hyper::{Body, Request};
use std::marker::PhantomData;
use std::task::{Context, Poll};

/// Middleware wrapper service that drops the context from the incoming request
/// and passes the plain `hyper::Request` to the wrapped service.
///
/// This service can be used to to include services that take a plain `hyper::Request`
/// in a `CompositeService` wrapped in an `AddContextService`.
///
/// Example Usage
/// =============
///
/// In the following example `SwaggerService` implements `hyper::service::MakeService`
/// with `Request = (hyper::Request, SomeContext)`, and `PlainService` implements it
/// with `Request = hyper::Request`
///
/// ```ignore
/// let swagger_service_one = SwaggerService::new();
/// let swagger_service_two = SwaggerService::new();
/// let plain_service = PlainService::new();
///
/// let mut composite_new_service = CompositeMakeService::new();
/// composite_new_service.push(("/base/path/1", swagger_service_one));
/// composite_new_service.push(("/base/path/2", swagger_service_two));
/// composite_new_service.push(("/base/path/3", DropContextMakeService::new(plain_service)));
/// ```
#[derive(Debug)]
pub struct DropContextMakeService {}

impl DropContextMakeService {
    /// Create a new DropContextMakeService struct wrapping a value
    pub fn new() -> Self {
        DropContextMakeService { }
    }
}

impl<T, C> hyper::service::Service<T> for DropContextMakeService {
    type Response = DropContextService<T, C>;
    type Error = std::io::Error;
    type Future = futures::future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, inner: T) -> Self::Future {
        futures::future::ok(DropContextService::new(inner))
    }
}

/// Swagger Middleware that wraps a `hyper::service::Service`, and drops any contextual information
/// on the request. Services will normally want to use `DropContextMakeService`, which will create
/// a `DropContextService` to handle each connection.
#[derive(Debug)]
pub struct DropContextService<T, C> {
    inner: T,
    marker: PhantomData<C>,
}

impl<T, C> DropContextService<T, C> {
    /// Create a new AddContextService struct wrapping a value
    pub fn new(inner: T) -> Self {
        DropContextService {
            inner,
            marker: PhantomData,
        }
    }
}

impl<T, C> hyper::service::Service<ContextualPayload<C>> for DropContextService<T, C>
    where
        C: Send + Sync + 'static,
        T: hyper::service::Service<Request<Body>>,
{
    type Response = T::Response;
    type Error = T::Error;
    type Future = T::Future;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: ContextualPayload<C>) -> Self::Future {
        self.inner.call(req.inner)
    }
}