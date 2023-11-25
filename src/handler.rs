use crate::controller::Controller;
use coap_lite::{CoapRequest, CoapResponse, RequestType, ResponseType};
use embassy_net::IpEndpoint;

const CONTROLLER_MTU: usize = 128;

pub async fn handle_request(
    request: CoapRequest<IpEndpoint>,
    controller: &mut Controller<'_>,
) -> Option<CoapResponse> {
    let method = request.get_method();
    let path = request.get_path();

    log::info!("request {method:?} {path}");

    match method {
        RequestType::Get => get(path.as_str(), request, controller).await,
        RequestType::Post => post(path.as_str(), request, controller).await,
        _ => {
            log::error!("unhandled method: {method:?}");
            request.response.map(|mut response| {
                response.set_status(ResponseType::MethodNotAllowed);
                response
            })
        }
    }
}

async fn get(
    path: &str,
    request: CoapRequest<IpEndpoint>,
    controller: &mut Controller<'_>,
) -> Option<CoapResponse> {
    match path {
        "uart" => {
            let mut buf = [0u8; CONTROLLER_MTU];

            match controller.read(&mut buf).await {
                Ok(len) => request.response.map(|mut response| {
                    response.message.payload = buf[..len].to_vec();
                    response
                }),
                Err(err) => {
                    log::error!("controller read failed: {err:?}");
                    request.response.map(|mut response| {
                        response.set_status(ResponseType::InternalServerError);
                        response
                    })
                }
            }
        }
        "version" => {
            const VERSION: &str = env!("CARGO_PKG_VERSION");

            request.response.map(|mut response| {
                response.message.payload = VERSION.as_bytes().to_vec();
                response
            })
        }
        _ => {
            log::error!("unhandled path: GET {path}");
            request.response.map(|mut response| {
                response.set_status(ResponseType::NotFound);
                response
            })
        }
    }
}

async fn post(
    path: &str,
    request: CoapRequest<IpEndpoint>,
    controller: &mut Controller<'_>,
) -> Option<CoapResponse> {
    match path {
        "uart" => {
            let payload = request.message.payload;

            if payload.len() > CONTROLLER_MTU {
                request.response.map(|mut response| {
                    response.set_status(ResponseType::BadRequest);
                    response
                })
            } else {
                match controller.write(payload.as_slice()).await {
                    Ok(len) => {
                        log::info!("Controller write {len} bytes.");
                        request.response
                    }
                    Err(err) => {
                        log::error!("Controller write failed: {err:?}");
                        request.response.map(|mut response| {
                            response.set_status(ResponseType::InternalServerError);
                            response
                        })
                    }
                }
            }
        }
        _ => {
            log::error!("unhandled path: POST {path}");
            request.response.map(|mut response| {
                response.set_status(ResponseType::NotFound);
                response
            })
        }
    }
}
