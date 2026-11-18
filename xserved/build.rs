// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     tonic_build::compile_protos("proto/calculator.proto")?;
//     Ok(())
// }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
    .compile_protos(
        &["proto/xserve.proto"],
        &["proto"],
    )?;
    Ok(())
}