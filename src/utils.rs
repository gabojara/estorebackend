use std::{
    future::{ready, Ready},
    rc::Rc,
};
use actix_web::{
    body::EitherBody,
    dev::{forward_ready,Service, ServiceRequest, ServiceResponse, Transform},
    http::header::AUTHORIZATION,
    Error, HttpMessage, HttpResponse,
};
use chrono::Utc;
use futures_util::future::LocalBoxFuture;
use dotenv::dotenv;
use std::env;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm };
use super::auth::model::Claims;

pub fn get_secret_key() -> String {
    dotenv().ok();
    env::var("JWT_SECRET")
        .expect("SECRET_KEY must be set")
}

pub struct Authentication;
 
impl<S, B> Transform<S, ServiceRequest> for Authentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static, // update here
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthenticationMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;
 
    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticationMiddleware {
            service: Rc::new(service), // convert S to Rc<S>
        }))
    }
}
 
pub struct AuthenticationMiddleware<S> {
    // service: S,
    service: Rc<S>,
}
 
impl<S, B> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static, // update here
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;
 
    forward_ready!(service);
 
    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        let secret = get_secret_key();
 
        Box::pin(async move {
            if let Some(auth_header) = req.headers().get(AUTHORIZATION) {
                if let Ok(auth_str)= auth_header.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        let token = &auth_str[7..];
                        let validation = Validation::new(Algorithm::HS256);
                        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
                        match decode::<Claims>(token, &decoding_key, &validation) {
                            Ok(token_data) => {
                                // Store the claims in a variable
                                let claims = token_data.claims;

                                let current_time = Utc::now().timestamp() as usize;

                                if claims.exp < current_time {
                                    let http_res = HttpResponse::Unauthorized().finish();
                                    let (http_req, _) = req.into_parts();
                                    let res = ServiceResponse::new(http_req, http_res);
                                    return Ok(res.map_into_right_body());
                                }

                                // Insert the claims into the request's extensions
                                req.extensions_mut().insert(claims);

                                // Continue with the next middleware / handler
                                let res = service.call(req).await?;
                                return Ok(res.map_into_left_body());
                            }
                            Err(_) => {
                                let http_res = HttpResponse::Unauthorized().finish();
                                let (http_req, _) = req.into_parts();
                                let res: ServiceResponse = ServiceResponse::new(http_req, http_res);
                                return Ok(res.map_into_right_body());
                            }
                        }
                    }
                }
            }
            // Getting some data here (just demo code for async function)
            let http_res = HttpResponse::Unauthorized().finish();
            let (http_req, _) = req.into_parts();
            let res: ServiceResponse = ServiceResponse::new(http_req, http_res);
            Ok(res.map_into_right_body())
        })
    }
}

// pub struct Authorization;

// impl<S, B> Transform<S, ServiceRequest> for Authorization
// where 
//     S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
//     S::Future: 'static,
//     B: 'static,
// {
//     type Response = ServiceResponse<EitherBody<B>>;
//     type Error = Error;
//     type InitError = ();
//     type Transform = AuthorizationMiddleware<S>;
//     type Future = Ready<Result<Self::Transform, Self::InitError>>;

//     fn new_transform(&self, service: S) -> Self::Future {
//         ready(Ok(AuthorizationMiddleware {
//             service: Rc::new(service),
//         }))
//     }
// }

// pub struct AuthorizationMiddleware<S> {
//     service: Rc<S>,
// }

// impl<S, B> Service<ServiceRequest> for AuthorizationMiddleware<S>
// where
//     S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
//     S::Future: 'static,
//     B: 'static,
// {
//     type Response = ServiceResponse<EitherBody<B>>;
//     type Error = Error;
//     type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

//     forward_ready!(service);

//     fn call(&self, req: ServiceRequest) -> Self::Future {
//         let service = Rc::clone(&self.service);

//         Box::pin(async move {
//             // Retrieve claims from the request's extensions
//             if let Some(claims) = req.extensions().get::<Claims>() {
//                 if claims.is_admin {
//                     // User is authorized, continue to the next middleware/handler
//                     let res = service.call(req).await?;
//                     return Ok(res.map_into_left_body());
//                 } else {
//                     // User is not authorized
//                     let http_res = HttpResponse::Forbidden().finish();
//                     let (http_req, _) = req.into_parts();
//                     let res: ServiceResponse = ServiceResponse::new(http_req, http_res);
//                     return Ok(res.map_into_right_body());
//                 }
//             }

//             // No claims found, unauthorized
//             let http_res = HttpResponse::Unauthorized().finish();
//             let (http_req, _) = req.into_parts();
//             let res: ServiceResponse = ServiceResponse::new(http_req, http_res);
//             Ok(res.map_into_right_body())
//         })
//     }
// }