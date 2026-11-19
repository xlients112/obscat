use tonic::{Request, Response, Status};

/// Helper untuk mengambil data murni dari gRPC Request tanpa mengetik .into_inner()
pub fn unpack<T>(req: Request<T>) -> T {
    req.into_inner()
}

/// Helper untuk membungkus data murni menjadi gRPC Response sukses (Ok)
pub fn pack<T>(res: T) -> Result<Response<T>, Status> {
    Ok(Response::new(res))
}

/// Helper untuk membuat error gRPC secara instan jika ada kegagalan internal
pub fn fail<T>(msg: &str) -> Result<Response<T>, Status> {
    Err(Status::internal(msg.to_string()))
}
