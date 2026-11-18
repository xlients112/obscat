use tonic::Request;

// helper membuat request client
pub fn to_req<T>(data: T) -> Request<T> {
    Request::new(data)
}