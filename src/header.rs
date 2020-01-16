use chrono::{DateTime, Utc};
use headers::{HeaderValue, HeaderName, Header, HeaderMapExt};
use lazy_static::lazy_static;
use std::convert::TryFrom;
use std::fmt;
use std::ops::Deref;
use uuid::Uuid;

/// Header - `X-Span-ID` - used to track a request through a chain of microservices.
pub const X_SPAN_ID: &str = "X-Span-ID";

lazy_static! {
    pub static ref X_SPAN_ID_HEADER: HeaderName = HeaderName::from_static(X_SPAN_ID);
}

/// Wrapper for a string being used as an X-Span-ID.
#[derive(Debug, Clone)]
pub struct XSpanId(pub String);

impl XSpanId {
    /// Extract an X-Span-ID from a request header if present, and if not
    /// generate a new one.
    pub fn get_or_generate<T>(req: &hyper::Request<T>) -> Self {
        let x_span_id = req.headers().typed_get::<XSpanId>();

        match x_span_id {
            Some(ref x) => x.clone(),
            None => Self::default(),
        }
    }
}

impl Header for XSpanId {
    fn name() -> &'static HeaderName {
        &X_SPAN_ID_HEADER
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
        where
            I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values
            .next()
            .ok_or_else(headers::Error::invalid)?;

        let value = value.to_str().map_err(|_| headers::Error::invalid())?;
        Ok(XSpanId(value.to_owned()))
    }

    fn encode<E>(&self, values: &mut E)
        where
            E: Extend<HeaderValue>,
    {
        let value = HeaderValue::from_str(&self.0.to_string()).unwrap();

        values.extend(std::iter::once(value));
    }
}

impl Default for XSpanId {
    fn default() -> Self {
        XSpanId(Uuid::new_v4().to_string())
    }
}

impl fmt::Display for XSpanId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A struct to allow homogeneous conversion into a HeaderValue. We can't
/// implement the From/Into trait on HeaderValue because we don't own
/// either of the types.
#[derive(Debug, Clone)]
pub struct IntoHeaderValue<T>(pub T);

// Generic implementations

impl<T> Deref for IntoHeaderValue<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

// Derive for each From<T> in hyper::header::HeaderValue

macro_rules! ihv_generate {
    ($t:ident) => {
        impl std::convert::TryFrom<HeaderValue> for IntoHeaderValue<$t> {
            type Error = headers::Error;
            fn try_from(hdr_value: HeaderValue) -> Result<Self, Self::Error> {
                let value = hdr_value.to_str().map_err(|_| headers::Error::invalid())?;
                let value = value.parse().map_err(|_| headers::Error::invalid())?;
                Ok(IntoHeaderValue(value))
            }
        }

        impl Into<HeaderValue> for IntoHeaderValue<$t> {
            fn into(self) -> HeaderValue {
                HeaderValue::from_str(&self.0.to_string()).unwrap()
            }
        }
    };
}

ihv_generate!(u64);
ihv_generate!(i64);
ihv_generate!(i16);
ihv_generate!(u16);
ihv_generate!(u32);
ihv_generate!(usize);
ihv_generate!(isize);
ihv_generate!(i32);

// Custom derivations

impl TryFrom<HeaderValue> for IntoHeaderValue<Vec<String>> {
    type Error = headers::Error;
    fn try_from(hdr_value: HeaderValue) -> Result<Self, Self::Error> {
        Ok(IntoHeaderValue(
            hdr_value
                .to_str()
                .map_err(|_| headers::Error::invalid())?
                .split(',')
                .filter_map(|x| match x.trim() {
                    "" => None,
                    y => Some(y.to_string()),
                })
                .collect(),
        ))
    }
}

impl Into<HeaderValue> for IntoHeaderValue<Vec<String>> {
    fn into(self) -> HeaderValue {
        HeaderValue::from_str(&self.0.join(", ")).unwrap()
    }
}

impl TryFrom<HeaderValue> for IntoHeaderValue<String> {
    type Error = headers::Error;
    fn try_from(hdr_value: HeaderValue) -> Result<Self, Self::Error> {
        let v = hdr_value
            .to_str()
            .map_err(|_| headers::Error::invalid())?
            .to_string();
        Ok(IntoHeaderValue(v))
    }
}

impl Into<HeaderValue> for IntoHeaderValue<String> {
    fn into(self) -> HeaderValue {
        HeaderValue::from_str(&self.0).unwrap()
    }
}

impl TryFrom<HeaderValue> for IntoHeaderValue<DateTime<Utc>> {
    type Error = headers::Error;
    fn try_from(hdr_value: HeaderValue) -> Result<Self, Self::Error> {
        let v = hdr_value.to_str().map_err(|_| headers::Error::invalid())?;
        Ok(IntoHeaderValue(
            DateTime::parse_from_rfc3339(v)
                .map_err(|_| headers::Error::invalid())?
                .with_timezone(&Utc)
        ))
    }
}

impl Into<HeaderValue> for IntoHeaderValue<DateTime<Utc>> {
    fn into(self) -> HeaderValue {
        HeaderValue::from_str(self.0.to_rfc3339().as_str()).unwrap()
    }
}
