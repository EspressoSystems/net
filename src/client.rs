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

pub async fn response_to_result<E: Error>(mut res: Response) -> surf::Result<Response> {
    if res.status() == StatusCode::Ok {
        Ok(res)
    } else {
        let err: E = response_body(&mut res).await?;
        Err(surf::Error::new(err.status(), err))
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
