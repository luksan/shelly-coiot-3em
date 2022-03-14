use anyhow::{Context, Result};
use serde::Deserialize;
use serde_tuple::Deserialize_tuple;
use serde_with::{serde_as, OneOrMany};

use std::collections::LinkedList;
use std::fmt::Debug;
use std::time::Duration;

#[derive(Copy, Clone, PartialEq)]
#[repr(u16)]
#[non_exhaustive]
pub enum CoIoTOption {
    GlobalDevId = 3332,
    StatusValidity = 3332 + 8 * (10 + 0),
    StatusSerial = 3332 + 8 * (10 + 1),
}

impl Into<u16> for CoIoTOption {
    fn into(self) -> u16 {
        self as u16
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Version {
    V1,
    V2,
    Unknown(u32),
}

#[derive(Clone, Debug)]
pub struct Response(pub coap_lite::Packet);

impl Response {
    pub fn device_id(&self) -> Option<&str> {
        self.get_opt(CoIoTOption::GlobalDevId)?
            .front()
            .map(|s| std::str::from_utf8(s).ok())?
    }

    pub fn device_serial(&self) -> Option<&str> {
        self.device_id()?
            .split_once('#')?
            .1
            .split_once('#')?
            .0
            .into()
    }

    pub fn device_type(&self) -> Option<&str> {
        self.device_id()?.split_once('#')?.0.into()
    }

    pub fn coiot_version(&self) -> Option<Version> {
        self.device_id().map(|i| {
            Some(match i.rsplit_once('#')?.1 {
                "1" => Version::V1,
                "2" => Version::V2,
                x => Version::Unknown(x.parse().ok()?),
            })
        })?
    }

    pub fn validity_duration(&self) -> Option<Duration> {
        let dur = u16::from_ne_bytes(
            self.get_opt(CoIoTOption::StatusValidity)?
                .front()?
                .get(..2)?
                .try_into()
                .unwrap(),
        );
        if dur & 1 == 1 {
            // dur is 4sec increments
            Some(Duration::from_secs(dur as u64 * 4))
        } else {
            // The given u16 representes 1/10 s
            Some(Duration::from_millis(dur as u64 * 100))
        }
    }

    pub fn msg_seq_no(&self) -> Option<u16> {
        self.get_opt(CoIoTOption::StatusSerial)?
            .front()?
            .get(..2)?
            .try_into()
            .ok()
            .map(u16::from_ne_bytes)
    }

    fn get_opt<O: Into<u16>>(&self, opt: O) -> Option<&LinkedList<Vec<u8>>> {
        let opt = opt.into();
        self.0.options().find(|(&o, _)| o == opt).map(|(_, x)| x)
    }

    pub fn deserialize_payload<'a, T: Deserialize<'a> + Debug>(&'a self) -> Result<T> {
        let payload_str =
            std::str::from_utf8(self.0.payload.as_slice()).context("Payload not valid utf8")?;
        // println!("{}", json::parse(payload_str).unwrap().pretty(2));
        serde_json::from_str(payload_str)
            .with_context(|| json::parse(payload_str).map(|j| j.pretty(2)).unwrap())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Description {
    blk: Vec<BlockDesc>,
    sen: Vec<SenDesc>,
}

type BlockId = u32;

#[derive(Deserialize, Debug, Clone)]
struct BlockDesc {
    #[serde(rename = "I")]
    id: BlockId,
    #[serde(rename = "D")]
    descr: String,
}

type SenId = u32;

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct SenDesc {
    #[serde(rename = "I")]
    id: SenId,
    #[serde(rename = "D")]
    descr: String,
    #[serde(rename = "T")]
    kind: SenType,
    #[serde(rename = "U")]
    unit: Option<String>,
    #[serde(rename = "R")]
    range: Option<RangeDesc>,
    #[serde(rename = "L")]
    #[serde_as(deserialize_as = "OneOrMany<_>")]
    links: Vec<BlockId>,
}

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct RangeDesc(#[serde_as(deserialize_as = "OneOrMany<_>")] Vec<String>);

/*
“A” 	Alarm
“B” 	Battery level
“C” 	Concentration
“E” 	Energy
“EV” 	Event
“EVC” 	Event counter
“H” 	Humidity
“I” 	Current
“L” 	Luminosity
“P” 	Power
“S” 	Status (catch-all if no other fits)
“T” 	Temperature
“V” 	Voltage
 */
#[derive(Deserialize, Debug, Clone)]
pub enum SenType {
    #[serde(rename = "A")]
    Alarm,
    #[serde(rename = "I")]
    Current,
    #[serde(rename = "E")]
    Energy,
    #[serde(rename = "EVC")]
    EventCounter,
    #[serde(rename = "P")]
    Power,
    #[serde(rename = "S")]
    Status,
    #[serde(rename = "V")]
    Voltage,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Status {
    #[serde(rename = "G")]
    generic: Vec<StatusEntry>,
}

impl Status {
    pub fn gen_entries(&self) -> impl Iterator<Item = StatusEntry> + '_ {
        self.generic.iter().copied()
    }
}

#[derive(Deserialize_tuple, Debug, Copy, Clone)]
pub struct StatusEntry {
    channel: u32,
    id: SenId,
    value: f64,
}

impl StatusEntry {
    pub fn pretty_print(&self, desc: &Description) {
        if let Some(d) = desc.sen.iter().find(|d| d.id == self.id) {
            let blk = desc
                .blk
                .iter()
                .filter(|blk| d.links.contains(&blk.id))
                .collect::<Vec<_>>();
            let multiple_devs: String;
            let device = match blk.len() {
                0 => "Unknown device",
                1 => blk[0].descr.as_ref(),
                _ => {
                    multiple_devs = blk.iter().map(|b| b.descr.as_str()).collect();
                    multiple_devs.as_ref()
                }
            };
            println!(
                "#{}: {device:8}: {:14} {:8.2} {} {:?}",
                self.id,
                d.descr,
                self.value,
                d.unit.as_deref().unwrap_or(""),
                d.range
            );
            //println!("{:?} : {:?}", self, d);
        } else {
            println!("No description for {:?} found.", self)
        }
    }
}
