#[macro_use]
extern crate rocket;
use rocket::State;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use dotenv::dotenv;
use thiserror::Error;
use modbus::Client;
use modbus::tcp;
use std::env;
use phf::phf_map;
use std::collections::HashMap;

struct MinerState {
    client_ip: String,
}

#[derive(Serialize, Clone, Copy)]
struct PhasePower {
    power: u32,
    #[serde(rename = "reactivePower")]
    reactive_power: u32,
    #[serde(rename = "apparentPower")]
    apparent_power: u32,
}

#[derive(Serialize)]
struct MinerData {
    #[serde(rename = "importedPowerTotal")]
    imported_power_total: u32,
    #[serde(rename = "importedReactivePowerTotal")]
    imported_reactive_power_total: u32,
    #[serde(rename = "exportedPowerTotal")]
    exported_power_total: u32,
    #[serde(rename = "exportedReactivePowerTotal")]
    exported_reactive_power_total: u32,

    phase1: PhasePower,
    phase2: PhasePower,
    phase3: PhasePower,
}

#[derive(Error,Debug)]
enum MinerError {
    #[error("Cannot read u32 from register {register:#x}: {message}")]
    BadDataRead{
        register: u16,
        message: String
    },
    #[error("Cannot open Modbus TCP transport to pull data: {0}")]
    ModbusTransportIssue(String),
}

#[derive(PartialEq, Eq, Hash)]
enum PhaseFields {
    Power,
    ApparentPower,
    ReactivePower,
}

static BASE_DATA_REGISTER: phf::Map<&'static str, u16> = phf_map! {
    "imported_power_total" => 0x34,
    "imported_reactive_power_total" => 0x36,
    "exported_power_total" => 0x4e,
    "exported_reactive_power_total" => 0x50,
};


/**
 * Polls for all of the desired data from the power analyzer.
 * 
 * This populates the entire struct and will trigger a large amount of TCP queries
 * on one individual connection to save connection energy.
 */
fn poll_solar_data(client_ip: String) -> Result<MinerData, MinerError> {
    // Create a client first.
    let mut client = tcp::Transport::new(&client_ip)
      .or_else(|e| Err(MinerError::ModbusTransportIssue(e.to_string())))?;

    // Create the struct we'll need, but first setup all the reads.
    // For the purposes of experimentation, we'll use our constant map to get
    // the keys for each value stored in a separate map and then add them to our enum.
    //
    // Let's make the base map first.
    let mut polled_data: HashMap<&'static str, u32> = HashMap::new();

    // We'll then make an extra array that stores the PhaseData for each phase.
    let mut phase_data: Vec<PhasePower> = Vec::new();

    // For all the base values, just add them to the map.
    for (field_name, register) in &BASE_DATA_REGISTER {
        let value = read_modbus_int32(&mut client, *register)
          .or_else(|e| Err(MinerError::BadDataRead { register: *register, message: e.to_string() }))?;

        polled_data.insert(*field_name, value);
    }

    // A map to more easily store the addresses for the phase power registers.
    let phase_data_registers: HashMap<PhaseFields, u16> = HashMap::from(
        [
         (PhaseFields::Power, 0x12),
         (PhaseFields::ApparentPower, 0x18),
         (PhaseFields::ReactivePower, 0x1e),
        ]
    );

    // For the three phases, just offset by 2 16-bit words for each value.
    // Go through each necessary field and just go to phase_0_register + i * 0x2
    // to get the value needed.
    for i in 0..3 {
        let mut power = 0;
        let mut apparent_power = 0;
        let mut reactive_power = 0;
        for (field_name, register) in &phase_data_registers {
            let value = read_modbus_int32(&mut client, register + i * 0x2)
                      .or_else(|e| Err(MinerError::BadDataRead { register: *register, message: e.to_string() }))?;
            match *field_name {
                PhaseFields::Power => power = value,
                PhaseFields::ApparentPower => apparent_power = value,
                PhaseFields::ReactivePower => reactive_power = value,
            }
        }

        phase_data.insert(i.into(), PhasePower {
            power: power,
            reactive_power: reactive_power,
            apparent_power: apparent_power
        });
    }

    // Close the client up.
    client.close().or_else(|e| Err(MinerError::ModbusTransportIssue(e.to_string())))?;

    // Unwraps are not nice here, but they're fine, because allegedly
    // all the fields in the map are populated at this point.
    Ok(MinerData{
        imported_power_total: *polled_data.get("imported_power_total").unwrap(),
        imported_reactive_power_total: *polled_data.get("imported_reactive_power_total").unwrap(),
        exported_power_total: *polled_data.get("exported_power_total").unwrap(),
        exported_reactive_power_total: *polled_data.get("exported_reactive_power_total").unwrap(),
        phase1: phase_data[0],
        phase2: phase_data[1],
        phase3: phase_data[2],
    })
}

fn read_modbus_int32(client: &mut tcp::Transport, register: u16) -> Result<u32, modbus::Error> {
    let mut array = client.read_holding_registers(register, 2)?;

    array.reverse();
    Ok((array[0] as u32) << 16 | (array[1] as u32))
}

#[get("/data")]
fn data(state: &State<MinerState>) -> Result<Json<MinerData>, Status> {
    let data = poll_solar_data(state.client_ip.clone())
      .or_else(|_| Err(Status::InternalServerError))?;
    Ok(Json(data))
}

#[launch]
fn rocket() -> _ {
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
