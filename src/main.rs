#![allow(dead_code)]

mod coiot;

use crate::coiot::Response;
use anyhow::{Context, Result};
use coap::CoAPClient;
use coap_lite::CoapResponse;
use coiot::{Description, Status};
use serde::Deserialize;
use std::fmt::Debug;
use std::io::Read;

fn get_json_payload<'b, 'a: 'b, T: Deserialize<'b> + Debug>(resp: &'a CoapResponse) -> Result<T> {
    let payload_str =
        std::str::from_utf8(resp.message.payload.as_slice()).context("Payload not valid utf8")?;
    // println!("{}", json::parse(payload_str).unwrap().pretty(2));
    serde_json::from_str(payload_str)
        .with_context(|| json::parse(payload_str).map(|j| j.pretty(2)).unwrap())
}

fn get_description() -> Result<Description> {
    let x = CoAPClient::get("coap://192.168.10.107/cit/d")?;
    get_json_payload(&x).context("Parsing CoIoT descriptors")
}

fn get_status() -> Result<Status> {
    let x = CoAPClient::get("coap://192.168.10.107/cit/s")?;
    let r = Response(x.message);
    r.deserialize_payload()
}

fn print_status() -> Result<()> {
    let desc = get_description()?;
    let status = get_status()?;
    for e in status.gen_entries() {
        e.pretty_print(&desc);
    }
    Ok(())
}

fn observe() -> Result<()> {
    let mut c = CoAPClient::new("192.168.10.107:5683")?;
    c.observe("/cit/s", |pkt| {
        let r = Response(pkt);
        println!("{:?}", r.deserialize_payload::<Status>())
    })
    .context("Failed to start observer.")?;
    let _ = std::io::stdin().read_to_string(&mut String::new());
    c.unobserve();
    Ok(())
}

fn main() -> Result<()> {
    print_status()?;
    //get_status()?;
    // observe()?;
    Ok(())
}
