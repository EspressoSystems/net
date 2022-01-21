use serde::{de::DeserializeOwned, Serialize};
use snafu::{ErrorCompat, IntoError};

/// Errors which can be serialized in a response body.
///
/// This trait can be used to define a standard error type returned by all API endpoints. When a
/// request fails for any reason, the body of the response will contain a serialization of
/// the error that caused the failure, upcasted into an anyhow::Error. If the error is an instance
/// of the standard error type for that particular API, it can be deserialized and downcasted to
/// this type on the client. The `client` module provides a middleware handler that does this
/// automatically.
///
/// Other errors (those which don't downcast to the API's errortype, such as errors
/// generated from the [tide] framework) will be serialized as strings using their [Display]
/// instance and encoded as an API error using the `catch_all` function.
pub trait Error: std::error::Error + Serialize + DeserializeOwned + Send + Sync + 'static {
    fn catch_all(msg: String) -> Self;
    fn status(&self) -> tide::StatusCode;

    /// Convert from a generic client-side error to a specific error type.
    ///
    /// If `source` can be downcast to `Self`, it is simply downcasted. Otherwise, it is converted
    /// to a [String] using [Display] and then converted to `Self` using [catch_all].
    fn from_client_error(source: surf::Error) -> Self {
        match source.downcast::<Self>() {
            Ok(err) => err,
            Err(err) => Self::catch_all(err.to_string()),
        }
    }
}

/// Convert a concrete error type into a server error.
///
/// The error is first converted into an `E` using the [From] instance. That error is then
/// upcasted into an anyhow error to be embedded in the [tide::Error], using the status code
/// indicated by [Error::status_code].
///
/// TODO the best way I can think of using this is something like
/// ```ignore
/// enum MyError { ... }
///
/// impl Error for MyError { ... }
///
/// fn my_error(error: impl Into<MyError>) -> tide::Error {
///     server_error(error)
/// }
///
/// fn some_endpoint(...) {
///     ...
///     some_result.map_err(my_error)?;
///     ...
/// }
/// ```
/// to ensure that the correct type parameter `MyError` is always used with `server_error`. A better
/// way would be to define a `Server` type which wraps a `tide` server, and takes endpoints of the
/// form `(...) -> Result<impl Serialize, impl Error>` and then calls `server_error` internal. This
/// would also be a good place to put other common server-related code, such as parsing api.toml and
/// route dispatching.
pub fn server_error<E: Error>(error: impl Into<E>) -> tide::Error {
    let error = error.into();
    tide::Error::new(error.status(), error)
}

/// Context for embedding network client errors into specific error types.
///
/// This type implements the [IntoError] trait from SNAFU, so it can be used with
/// [ResultExt::context] just like automatically generated SNAFU contexts.
///
/// Calling `some_result.context(ClientError)` will convert a potential error from a [surf::Error]
/// to a specific error type `E` using the method `E::from_client_error`, provided by the
/// [Error] trait.
///
/// This is the inverse of [server_error], and can be used on the client side to recover errors
/// which were generated on the server using [server_error].
pub struct ClientError;

impl<E: Error + ErrorCompat + std::error::Error> IntoError<E> for ClientError {
    type Source = surf::Error;

    fn into_error(self, source: Self::Source) -> E {
        E::from_client_error(source)
    }
}

/// Convert a concrete error type into a client error.
///
/// The error is first converted into an [Error] using the [Into] instance. That error is then
/// upcasted into an anyhow error to be embedded in the [surf::Error].
///
/// This is the equivalent for [server_error] for errors generated on the client side; for instance,
/// in middleware.
pub fn client_error<E: Error>(error: impl Into<E>) -> surf::Error {
    let error = error.into();
    surf::Error::new(error.status(), error)
}
