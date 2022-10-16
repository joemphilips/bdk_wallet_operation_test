use std::error::Error;
use clap::Parser;
use wallet_operation_test::send_bitcoin::wallet_send_tx;

#[derive(Debug, Parser)]
#[clap(
    name = ""
)]
struct CliArg {
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    wallet_send_tx().await
}