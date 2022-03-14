use serde::Deserialize;
use serde_tuple::Deserialize_tuple;
use serde_with::{serde_as, OneOrMany};

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
    #[serde(rename = "R", flatten)]
    range: Option<RangeDesc>,
    #[serde(rename = "L")]
    #[serde_as(deserialize_as = "OneOrMany<_>")]
    links: Vec<BlockId>,
}

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct RangeDesc {
    #[serde_as(deserialize_as = "OneOrMany<_>")]
    range: Vec<String>,
}

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
                "#{}: {device:8}: {:14} {:8.2} {}",
                self.id,
                d.descr,
                self.value,
                d.unit.as_deref().unwrap_or("")
            );
            //println!("{:?} : {:?}", self, d);
        } else {
            println!("No description for {:?} found.", self)
        }
    }
}
