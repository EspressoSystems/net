// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the Net library.

// This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::error::Error;
use futures::future::BoxFuture;
use mime::Mime;
use serde::{Deserialize, Serialize};
use tide::http::{content::Accept, mime};
use tide::{Body, Next, Request, Response, StatusCode};
use tracing::{event, Level};

/// Deserialize the body of a request.
///
/// The Content-Type header is used to determine the serialization format.
pub async fn request_body<T: for<'de> Deserialize<'de>, S>(
    req: &mut Request<S>,
) -> Result<T, tide::Error> {
    if let Some(content_type) = req.header("Content-Type") {
        match content_type.as_str() {
            "application/json" => req.body_json().await,
            "application/octet-stream" => {
                let bytes = req.body_bytes().await?;
                bincode::deserialize(&bytes).map_err(|err| {
                    tide::Error::from_str(
                        StatusCode::BadRequest,
                        format!("unable to deserialie request body: {}", err),
                    )
                })
            }
            content_type => Err(tide::Error::from_str(
                StatusCode::BadRequest,
                format!("unsupported content type {}", content_type),
            )),
        }
    } else {
        Err(tide::Error::from_str(
            StatusCode::BadRequest,
            "unspecified content type",
        ))
    }
}

pub fn best_response_type(
    accept: &mut Option<Accept>,
    available: &[Mime],
) -> Result<Mime, tide::Error> {
    match accept {
        Some(accept) => {
            // The Accept type has a `negotiate` method, but it doesn't properly handle
            // wildcards. It handles * but not */* and basetype/*, because for content type
            // proposals like */* and basetype/*, it looks for a literal match in `available`,
            // it does not perform pattern matching. So, we implement negotiation ourselves.
            //
            // First sort by the weight parameter, which the Accept type does do correctly.
            accept.sort();
            // Go through each proposed content type, in the order specified by the client, and
            // match them against our available types, respecting wildcards.
            for proposed in accept.iter() {
                if proposed.basetype() == "*" {
                    // The only acceptable Accept value with a basetype of * is */*, therefore
                    // this will match any available type.
                    return Ok(available[0].clone());
                } else if proposed.subtype() == "*" {
                    // If the subtype is * but the basetype is not, look for a proposed type
                    // with a matching basetype and any subtype.
                    for mime in available {
                        if mime.basetype() == proposed.basetype() {
                            return Ok(mime.clone());
                        }
                    }
                } else if available.contains(proposed) {
                    // If neither part of the proposal is a wildcard, look for a literal match.
                    return Ok((**proposed).clone());
                }
            }

            if accept.wildcard() {
                // If no proposals are available but a wildcard flag * was given, return any
                // available content type.
                Ok(available[0].clone())
            } else {
                Err(tide::Error::from_str(
                    StatusCode::NotAcceptable,
                    "No suitable Content-Type found",
                ))
            }
        }
        None => {
            // If no content type is explicitly requested, default to the first available type.
            Ok(available[0].clone())
        }
    }
}

fn respond_with<T: Serialize>(
    accept: &mut Option<Accept>,
    body: T,
) -> Result<Response, tide::Error> {
    let ty = best_response_type(accept, &[mime::JSON, mime::BYTE_STREAM])?;
    if ty == mime::BYTE_STREAM {
        let bytes = bincode::serialize(&body)?;
        Ok(Response::builder(tide::StatusCode::Ok)
            .body(bytes)
            .content_type(mime::BYTE_STREAM)
            .build())
    } else if ty == mime::JSON {
        Ok(Response::builder(tide::StatusCode::Ok)
            .body(Body::from_json(&body)?)
            .content_type(mime::JSON)
            .build())
    } else {
        unreachable!()
    }
}

/// Serialize the body of a response.
///
/// The Accept header of the request is used to determine the serialization format.
///
/// This function combined with the [add_error_body] middleware defines the server-side protocol
/// for encoding espresso types in HTTP responses.
pub fn response<T: Serialize, S>(req: &Request<S>, body: T) -> Result<Response, tide::Error> {
    respond_with(&mut Accept::from_headers(req)?, body)
}

/// Server middleware which automatically populates the body of error responses.
///
/// If the response contains an error, the error is encoded into the [Error] type (either by
/// downcasting if the server has generated an instance of [Error], or by converting to a
/// [String] using [Display] if the error can not be downcasted to [Error]). The resulting
/// [Error] is then serialized and used as the body of the response.
///
/// If the response does not contain an error, it is passed through unchanged.
///
/// This middleware is the inverse of the client-side middleware `parse_error_body`, which
/// automatically converts error responses into [Err] variants, assuming the responses follow
/// the convention implemented by this middleware.
pub fn add_error_body<'a, T: Clone + Send + Sync + 'static, E: Error>(
    req: Request<T>,
    next: Next<'a, T>,
) -> BoxFuture<'a, tide::Result> {
    Box::pin(async {
        let mut accept = Accept::from_headers(&req)?;
        let mut res = next.run(req).await;
        if let Some(error) = res.take_error() {
            let error = E::from_client_error(error);
            event!(Level::WARN, "responding with error: {}", error);
            let mut res = respond_with(&mut accept, &error)?;
            res.set_status(error.status());
            Ok(res)
        } else {
            Ok(res)
        }
    })
}

/// Server middleware which logs requests and responses.
pub fn trace<'a, T: Clone + Send + Sync + 'static>(
    req: tide::Request<T>,
    next: tide::Next<'a, T>,
) -> BoxFuture<'a, tide::Result> {
    Box::pin(async {
        event!(
            Level::INFO,
            "<-- received request {{url: {}, content-type: {:?}, accept: {:?}}}",
            req.url(),
            req.content_type(),
            Accept::from_headers(&req),
        );
        let res = next.run(req).await;
        event!(
            Level::INFO,
            "--> responding with {{content-type: {:?}, error: {:?}}}",
            res.content_type(),
            res.error(),
        );
        Ok(res)
    })
}
