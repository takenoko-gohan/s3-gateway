use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_sdk_s3::Client;
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::StatusCode;

type Response = hyper::Response<BoxBody<Bytes, hyper::Error>>;
type Error = Box<dyn std::error::Error + Send + Sync>;

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

pub fn easy_response(status_code: StatusCode) -> Result<Response, Error> {
    let body = match status_code {
        StatusCode::OK => full("OK"),
        StatusCode::BAD_REQUEST => full("Bad Request"),
        StatusCode::NOT_FOUND => full("Not Found"),
        StatusCode::METHOD_NOT_ALLOWED => full("Method Not Allowed"),
        StatusCode::INTERNAL_SERVER_ERROR => full("Internal Server Error"),
        _ => full("Unknown"),
    };

    Ok(hyper::Response::builder()
        .header("Content-Type", mime::TEXT_PLAIN.to_string())
        .status(status_code)
        .body(body)?)
}

pub async fn s3_object_response(
    s3_client: Client,
    bucket: &str,
    key: &str,
    no_such_key_redirect_path: Option<String>,
) -> Result<Response, Error> {
    let s3_obj = match s3_client.get_object().bucket(bucket).key(key).send().await {
        Ok(obj) => obj,
        Err(e) => return get_s3_object_error(e, no_such_key_redirect_path),
    };

    let b = match s3_obj.body.collect().await {
        Ok(b) => b.into_bytes(),
        Err(e) => {
            tracing::error!("Failed to collect body: {:?}", e);
            return easy_response(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let content_type = mime_guess::from_path(key)
        .first_or(mime::TEXT_PLAIN)
        .to_string();
    Ok(hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .body(full(b))?)
}

fn get_s3_object_error(
    error: SdkError<GetObjectError>,
    no_such_key_redirect_path: Option<String>,
) -> Result<Response, Error> {
    tracing::warn!("Failed to get object: {:?}", error);
    if error.into_service_error().is_no_such_key() {
        match no_such_key_redirect_path {
            Some(redirect_path) => Ok(hyper::Response::builder()
                .status(StatusCode::FOUND)
                .header("Content-Type", mime::TEXT_PLAIN.to_string())
                .header("Location", redirect_path)
                .body(full("Found"))?),
            None => easy_response(StatusCode::NOT_FOUND),
        }
    } else {
        easy_response(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

#[cfg(test)]
mod tests {
    use hyper::StatusCode;
    use pretty_assertions::assert_eq;
    use test_case::test_case;

    #[test_case(StatusCode::OK)]
    #[test_case(StatusCode::BAD_REQUEST)]
    #[test_case(StatusCode::NOT_FOUND)]
    #[test_case(StatusCode::INTERNAL_SERVER_ERROR)]
    #[tokio::test]
    async fn test_easy_response(status_code: StatusCode) {
        let res = super::easy_response(status_code);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().status(), status_code);
    }
}
