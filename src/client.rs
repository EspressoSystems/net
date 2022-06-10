// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the Net library.

// This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::error::Error;
use futures::future::BoxFuture;
use futures::prelude::*;
use serde::Deserialize;
use surf::{middleware::Next, Client, Request, Response, StatusCode};

/// Deserialize the body of a response.
///
/// The Content-Type header is used to determine the serialization format.
///
/// This function combined with the [parse_error_body] middleware defines the client-side
/// protocol for decoding espresso types from HTTP responses.
pub async fn response_body<T: for<'de> Deserialize<'de>>(
    res: &mut Response,
) -> Result<T, surf::Error> {
    if let Some(content_type) = res.header("Content-Type") {
        match content_type.as_str() {
            "application/json" => res.body_json().await,
            "application/octet-stream" => {
                bincode::deserialize(&res.body_bytes().await?).map_err(|err| {
                    surf::Error::from_str(
                        StatusCode::InternalServerError,
                        format!("response body fails to deserialize: {}", err),
                    )
                })
            }
            content_type => Err(surf::Error::from_str(
                StatusCode::UnsupportedMediaType,
                format!("unsupported content type {}", content_type),
            )),
        }
    } else {
        Err(surf::Error::from_str(
            StatusCode::UnsupportedMediaType,
            "unspecified content type in response",
        ))
    }
}

async fn response_error<E: Error>(res: &mut Response) -> E {
    // To add context to the error, try to interpret the response body as a serialized error. Since
    // `body_json`, `body_string`, etc. consume the response body, we will extract the body as raw
    // bytes and then try various potential decodings based on the response headers and the contents
    // of the body.
    let bytes = match res.body_bytes().await {
        Ok(bytes) => bytes,
        Err(err) => {
            // If we are unable to even read the body, just return a generic error message based on
            // the status code.
            return E::catch_all(format!(
                "Request terminated with error {}. Failed to read request body due to {}",
                res.status(),
                err
            ));
        }
    };
    if let Some(content_type) = res.header("Content-Type") {
        // If the response specifies a content type, check if it is one of the types we know how to
        // deserialize, and if it is, we can then see if it deserializes to an `E`.
        match content_type.as_str() {
            "application/json" => {
                if let Ok(err) = serde_json::from_slice(&bytes) {
                    return err;
                }
            }
            "application/octet-stream" => {
                if let Ok(err) = bincode::deserialize(&bytes) {
                    return err;
                }
            }
            _ => {}
        }
    }
    // If we get here, then we were not able to interpret the response body as an `E` directly. This
    // can be because:
    //  * the content type is not supported for deserialization
    //  * the content type was unspecified
    //  * the body did not deserialize to an `E`
    // We have one thing left we can try: if the body is a string, we can use the `catch_all`
    // variant of `E` to include the contents of the string in the error message.
    if let Ok(msg) = std::str::from_utf8(&bytes) {
        return E::catch_all(msg.to_string());
    }

    // The response body was not an `E` or a string. Return the most helpful error message we can,
    // including the status code, content type, and raw body.
    E::catch_all(format!(
        "Request terminated with error {}. Content-Type: {}. Body: 0x{}",
        res.status(),
        match res.header("Content-Type") {
            Some(content_type) => content_type.as_str(),
            None => "unspecified",
        },
        hex::encode(&bytes)
    ))
}

pub async fn response_to_result<E: Error>(mut res: Response) -> surf::Result<Response> {
    if res.status() == StatusCode::Ok {
        Ok(res)
    } else {
        let err = response_error::<E>(&mut res).await;
        Err(surf::Error::new(res.status(), err))
    }
}

/// Client middleware which turns responses with non-success statuses into errors.
///
/// If the status code of the response is Ok (200), the response is passed through unchanged.
/// Otherwise, the body of the response is treated as an [Error] which is lifted into a
/// [surf::Error]. This can then be converted into a module-specific error type using
/// [FromApiError::from_client_error].
///
/// If the request fails without producing a response at all, the [surf::Error] from the failed
/// request is passed through.
///
/// This middleware is the inverse of the server-side middleware `add_error_body`, which
/// automatically prepares the body of error responses for interpretation by this client side
/// middleware.
pub fn parse_error_body<E: Error>(
    req: Request,
    client: Client,
    next: Next<'_>,
) -> BoxFuture<surf::Result<Response>> {
    Box::pin(
        next.run(req, client)
            .and_then(|res| async { response_to_result::<E>(res).await }),
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use serde::{Deserialize, Serialize};
    use snafu::Snafu;
    use surf::http::{self, mime, Body};

    #[derive(Clone, Debug, Deserialize, Serialize, Snafu, PartialEq, Eq)]
    struct Error {
        msg: String,
    }

    impl crate::Error for Error {
        fn catch_all(msg: String) -> Self {
            Self { msg }
        }

        fn status(&self) -> StatusCode {
            StatusCode::InternalServerError
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
    struct Data {
        field: u32,
    }

    #[async_std::test]
    async fn test_response_body_json() {
        let data = Data::default();
        let mut res = http::Response::new(StatusCode::Ok);
        res.set_content_type(mime::JSON);
        res.set_body(Body::from_json(&data).unwrap());

        // Convert the resopnse to a result, check that it is Ok, and deserialize the body.
        let mut res = response_to_result::<Error>(res.into()).await.unwrap();
        assert_eq!(data, response_body(&mut res).await.unwrap());
    }

    #[async_std::test]
    async fn test_response_body_bincode() {
        let data = Data::default();
        let mut res = http::Response::new(StatusCode::Ok);
        res.set_content_type(mime::BYTE_STREAM);
        res.set_body(bincode::serialize(&data).unwrap());

        // Convert the resopnse to a result, check that it is Ok, and deserialize the body.
        let mut res = response_to_result::<Error>(res.into()).await.unwrap();
        assert_eq!(data, response_body(&mut res).await.unwrap());
    }

    #[async_std::test]
    async fn test_response_error_json() {
        let msg = "This is an error message".to_string();
        let err = Error { msg };
        let mut res = http::Response::new(StatusCode::InternalServerError);
        res.set_content_type(mime::JSON);
        res.set_body(Body::from_json(&err).unwrap());

        // Convert the response to a result, check that it is Err, and deserialize the body.
        let res = response_to_result::<Error>(res.into()).await.unwrap_err();
        assert_eq!(err, res.downcast().unwrap());
    }

    #[async_std::test]
    async fn test_response_error_bincode() {
        let msg = "This is an error message".to_string();
        let err = Error { msg };
        let mut res = http::Response::new(StatusCode::InternalServerError);
        res.set_content_type(mime::BYTE_STREAM);
        res.set_body(bincode::serialize(&err).unwrap());

        // Convert the response to a result, check that it is Err, and deserialize the body.
        let res = response_to_result::<Error>(res.into()).await.unwrap_err();
        assert_eq!(err, res.downcast().unwrap());
    }

    #[async_std::test]
    async fn test_response_error_plaintext() {
        let msg = "This is an error message".to_string();
        let mut res = http::Response::new(StatusCode::InternalServerError);
        res.set_body(msg.clone());

        // Convert the response to a result, check that it is Err, and check that the error message
        // matches `msg`.
        let res = response_to_result::<Error>(res.into()).await.unwrap_err();
        let err: Error = res.downcast().unwrap();
        assert_eq!(err.msg, msg);
    }

    #[async_std::test]
    async fn test_response_error_html() {
        let msg = "<p>This is an error message</p>".to_string();
        let mut res = http::Response::new(StatusCode::InternalServerError);
        res.set_content_type(mime::HTML);
        res.set_body(msg.clone());

        // Convert the response to a result, check that it is Err, and check that the error message
        // matches `msg`.
        let res = response_to_result::<Error>(res.into()).await.unwrap_err();
        let err: Error = res.downcast().unwrap();
        assert_eq!(err.msg, msg);
    }

    #[async_std::test]
    async fn test_response_error_invalid_json() {
        let json = r#"{"error": "this is an error message"}"#;
        let mut res = http::Response::new(StatusCode::InternalServerError);
        res.set_content_type(mime::JSON);
        res.set_body(json);

        // Convert the response to a result, check that it is Err, and check that the response body
        // was interpreted as a string error message, since it does not deserialize to an `Error`.
        let res = response_to_result::<Error>(res.into()).await.unwrap_err();
        let err: Error = res.downcast().unwrap();
        assert_eq!(err.msg, json);
    }

    #[async_std::test]
    async fn test_response_error_bytes() {
        // Some bytes that are not valid UTF-8.
        let bytes = [0xC0u8, 0x7Fu8];
        assert!(std::str::from_utf8(&bytes).is_err());

        let mut res = http::Response::new(StatusCode::InternalServerError);
        res.set_content_type(mime::BYTE_STREAM);
        res.set_body(bytes.as_slice());

        // Convert the response to a result, check that it is Err, and check that the binary body is
        // encoded in the error message.
        let res = response_to_result::<Error>(res.into()).await.unwrap_err();
        let err: Error = res.downcast().unwrap();
        assert_eq!(err.msg, "Request terminated with error 500. Content-Type: application/octet-stream. Body: 0xc07f");
    }
}
