#[macro_use]
extern crate rocket;
use rocket::State;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use dotenv::dotenv;
use modbus::Client;
use modbus::tcp;
use std::env;

fn read_modbus_int32(client_ip: String, register: u16) -> Result<u32, modbus::Error> {
    let mut client = match tcp::Transport::new(&client_ip) {
        Ok(client) => client,
        Err(error) => panic!("Cannot create TCP transport for Modbus protocol: {}", error),
    };
    let mut array = client.read_holding_registers(register, 2)?;
    client.close().ok();
    array.reverse();

    Ok((array[0] as u32) << 16 | (array[1] as u32))
}

fn read_modbus_int16(client_ip: String, register: u16) -> Result<u16, modbus::Error> {
    let mut client = match tcp::Transport::new(&client_ip) {
        Ok(client) => client,
        Err(error) => panic!("Cannot create TCP transport for Modbus protocol: {}", error),
    };
    let value = client.read_holding_registers(register, 1)?[0];
    client.close().ok();
    Ok(value)
}

struct MinerState {
    client_ip: String,
}

#[derive(Serialize)]
struct MinerData {
    #[serde(rename = "firstRegister")]
    first_register: u32,
}

#[get("/data")]
fn data(state: &State<MinerState>) -> Result<Json<MinerData>, Status> {
    let first_register_data = read_modbus_int32(state.client_ip.clone(), 0x34).or_else(|_| Err(Status::InternalServerError))?;
    let data = MinerData{
        first_register: first_register_data,
    };
    Ok(Json(data))
}

#[launch]
fn rocket() -> _ {
    // For starters, we'll just get the Modbus implementation working
    // so that we can actually read some registers and print to stdout.
    dotenv().ok();

    let client_ip = match env::var("POWER_ANALYZER_IP") {
        Ok(value) => value,
        Err(error) => panic!("Cannot get environment variable for power analyzer's IP: {}", error),
    };

    let state = MinerState{ client_ip: client_ip };
    rocket::build()
      .mount("/", routes![data])
      .manage(state)
}
