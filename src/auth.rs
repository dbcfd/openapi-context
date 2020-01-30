//! Authentication and authorization data structures

use crate::{Push, ContextualPayload};
use hyper::HeaderMap;
use std::collections::BTreeSet;
use std::string::ToString;
use std::marker::PhantomData;
use std::task::{Context, Poll};

/// Authorization scopes.
#[derive(Clone, Debug, PartialEq)]
pub enum Scopes {
    /// Some set of scopes.
    Some(BTreeSet<String>),
    /// All possible scopes, authorization checking disabled.
    All,
}

/// Storage of authorization parameters for an incoming request, used for
/// REST API authorization.
#[derive(Clone, Debug, PartialEq)]
pub struct Authorization {
    /// Subject for which authorization is granted
    /// (i.e., what may be accessed.)
    pub subject: String,

    /// Scopes for which authorization is granted
    /// (i.e., what types of access are permitted).
    pub scopes: Scopes,

    /// Identity of the party to whom authorization was granted, if available
    /// (i.e., who is responsible for the access).
    ///
    /// In an OAuth environment, this is the identity of the client which
    /// issued an authorization request to the resource owner (end-user),
    /// and which has been directly authorized by the resource owner
    /// to access the protected resource. If the client delegates that
    /// authorization to another service (e.g., a proxy or other delegate),
    /// the `issuer` is still the original client which was authorized by
    /// the resource owner.
    pub issuer: Option<String>,
}

/// Storage of raw authentication data, used both for storing incoming
/// request authentication, and for authenticating outgoing client requests.
#[derive(Clone, Debug, PartialEq)]
pub enum AuthData {
    /// HTTP Basic auth.
    Basic(headers::Authorization<headers::authorization::Basic>),
    /// HTTP Bearer auth, used for OAuth2.
    Bearer(headers::Authorization<headers::authorization::Bearer>),
    /// Header-based or query parameter-based API key auth.
    ApiKey(String),
}

impl AuthData {
    /// Set Basic authentication
    pub fn basic(username: &str, password: &str) -> Self {
        AuthData::Basic(headers::Authorization::basic(
            username,
            password,
        ))
    }

    /// Set Bearer token authentication
    pub fn bearer(token: &str) -> Self {
        AuthData::Bearer(headers::Authorization::bearer(token).unwrap())
    }

    /// Set ApiKey authentication
    pub fn apikey(apikey: &str) -> Self {
        AuthData::ApiKey(apikey.to_owned())
    }
}

/// Bound for Request Context for MakeService wrappers
pub trait RcBound: Push<Option<Authorization>> + Send + Sync + 'static {}

impl<T> RcBound for T where T: Push<Option<Authorization>> + Send + Sync + 'static {}

/// Retrieve an API key from a header
pub fn api_key_from_header(headers: &HeaderMap, header: &str) -> Option<String> {
    headers
        .get(header)
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string)
}

/// Dummy Authenticator, that blindly inserts authorization data, allowing all
/// access to an endpoint with the specified subject.
#[derive(Debug)]
pub struct AllowAllAuthenticatorMakeService<C> {
    subject: String,
    phantom: PhantomData<C>,
}

impl<C> AllowAllAuthenticatorMakeService<C> {
    /// Create a new AddContextMakeService struct wrapping a value
    pub fn new<T: Into<String>>(subject: T) -> Self {
        AllowAllAuthenticatorMakeService {
            subject: subject.into(),
            phantom: PhantomData,
        }
    }
}

impl<T, C> hyper::service::Service<T> for AllowAllAuthenticatorMakeService<C> {
    type Response = AllowAllAuthenticator<T, C>;
    type Error = std::io::Error;
    type Future = futures::future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, inner: T) -> Self::Future {
        futures::future::ok(AllowAllAuthenticator::new(inner, self.subject.clone()))
    }
}

/// Middleware wrapper service, that should be used as the outermost layer in a
/// stack of hyper services. Adds a context to a plain `hyper::Request` that can be
/// used by subsequent layers in the stack. The `AddContextService` struct should
/// not usually be used directly - when constructing a hyper stack use
/// `AddContextMakeService`, which will create `AddContextService` instances as needed.
#[derive(Debug)]
pub struct AllowAllAuthenticator<T, C> {
    inner: T,
    subject: String,
    marker: PhantomData<C>,
}

impl<T, C> AllowAllAuthenticator<T, C> {
    /// Create a new AddContextService struct wrapping a value
    pub fn new<U: Into<String>>(inner: T, subject: U) -> Self {
        AllowAllAuthenticator {
            inner: inner,
            subject: subject.into(),
            marker: PhantomData,
        }
    }
}

impl<T, C> hyper::service::Service<ContextualPayload<C>> for AllowAllAuthenticator<T, C>
    where
        C: RcBound,
        C::Result: Send + Sync + 'static,
        T: hyper::service::Service<ContextualPayload<C::Result>>,
{
    type Response = T::Response;
    type Error = T::Error;
    type Future = T::Future;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: ContextualPayload<C>) -> Self::Future {
        let auth = Authorization {
            subject: self.subject.clone(),
            scopes: Scopes::All,
            issuer: None,
        };
        let context = req.context.push(Some(auth));

        self.inner.call(ContextualPayload {
            inner: req.inner,
            context: context,
        })
    }
}
