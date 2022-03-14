#![allow(dead_code)]

mod coiot;
use crate::coiot::{Description, Response, Status};

use anyhow::{Context, Result};
use coap::CoAPClient;
use coap_lite::{CoapResponse, MessageClass, Packet};
use serde::Deserialize;

use std::fmt::Debug;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

fn get_json_payload<'b, 'a: 'b, T: Deserialize<'b> + Debug>(resp: &'a CoapResponse) -> Result<T> {
    let payload_str =
        std::str::from_utf8(resp.message.payload.as_slice()).context("Payload not valid utf8")?;
    // println!("{}", json::parse(payload_str).unwrap().pretty(2));
    serde_json::from_str(payload_str)
        .with_context(|| json::parse(payload_str).map(|j| j.pretty(2)).unwrap())
}

fn get_description(host: SocketAddr) -> Result<Description> {
    let x = CoAPClient::get(&format!("coap://{}/cit/d", host))?;
    get_json_payload(&x).context("Parsing CoIoT descriptors")
}

fn get_status() -> Result<Status> {
    let x = CoAPClient::get("coap://192.168.10.107/cit/s")?;
    let r = Response(x.message);
    r.deserialize_payload()
}

fn print_status() -> Result<()> {
    let desc = get_description("192.168.10.107:5683".parse().unwrap())?;
    let status = get_status()?;
    for e in status.gen_entries() {
        e.pretty_print(&desc);
    }
    Ok(())
}

fn observe() -> Result<()> {
    let sock = UdpSocket::bind("0.0.0.0:5683").context("Failed to bind UPD socket")?;
    sock.join_multicast_v4(
        &"224.0.1.187".parse().unwrap(),
        &"192.168.10.223".parse().unwrap(),
    )
    .context("Failed to join multicast.")?;
    sock.set_read_timeout(Some(Duration::from_secs(20)))?;
    let mut buf = [0; 1500];
    loop {
        let (len, addr) = sock.recv_from(&mut buf).context("Failed to receive.")?;
        println!("Received from {}", addr);
        if addr.port() == 5683 {
            let pkt = Packet::from_bytes(&buf[..len])?;
            let r = Response(pkt);
            if r.0.header.code == MessageClass::from(30) {
                // Shelly non-std CoAP code
                let desc = get_description(addr)?;
                r.deserialize_payload::<Status>()?.pretty_print(&desc);
            }
        }
    }
}

fn main() -> Result<()> {
    print_status()?;
    //get_status()?;
    observe()?;
    Ok(())
}
