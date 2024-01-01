use edge_net::captive::{DnsConf, DnsServer};
use esp_idf_hal::{peripheral::Peripheral, prelude::Peripherals};
use esp_idf_svc::http::Method::Get;
use esp_idf_svc::wifi::{AccessPointConfiguration, AuthMethod, Configuration};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::server::EspHttpServer,
    nvs::{EspDefaultNvsPartition, EspNvsPartition, NvsDefault},
    wifi::{BlockingWifi, EspWifi},
};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use log::*;

const SSID: &str = env!("RUST_ESP32_STD_DEMO_WIFI_SSID");
const PASS: &str = env!("RUST_ESP32_STD_DEMO_WIFI_PASS");

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Hello, world!");

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take().unwrap();
    let _wifi = wifi(
        peripherals.modem,
        sysloop,
        Some(EspDefaultNvsPartition::take().unwrap()),
    )
    .unwrap();

    let mut server = EspHttpServer::new(&esp_idf_svc::http::server::Configuration {
        http_port: 80,
        uri_match_wildcard: true,
        ..Default::default()
    })?;

    server.fn_handler("/*", Get, move |mut req| {
        let uri = req.uri();
        let host1 = req.header("host");
        let host2 = req.header("host");
        info!("host1 {:?} host2 {:?}", host1, host2);
        let mut file_parameter_iterator = uri.split("file=");
        // First call to `next` will always be somthing
        file_parameter_iterator.next();
        // This call will either be None because file= wasn't in the uri or Some(value)
        let file_parameter = file_parameter_iterator.next();
        let file_parameter = match file_parameter {
            Some(parameter) => parameter.split('&').next(),
            _ => file_parameter,
        };
        info!("uri {:?}", file_parameter);
        let mut res = req.into_ok_response()?;
        res.write(home_page().as_bytes())?;
        Ok(())
    })?;

    let mut dns_conf = DnsConf::new("192.168.71.1".parse()?);
    dns_conf.bind_port = 53;
    let mut dns_server = DnsServer::new(dns_conf);
    dns_server.start()?;

    std::mem::forget(_wifi);
    std::mem::forget(server);
    std::mem::forget(dns_server);
    Ok(())
}

pub fn wifi(
    modem: impl Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
    nvs: Option<EspNvsPartition<NvsDefault>>,
) -> anyhow::Result<BlockingWifi<EspWifi<'static>>> {
    let esp_wifi = EspWifi::new(modem, sysloop.clone(), nvs)?;
    let mut wifi = BlockingWifi::wrap(esp_wifi, sysloop)?;

    connect_wifi(&mut wifi)?;

    let ip_info = wifi.wifi().ap_netif().get_ip_info();

    println!("Wifi DHCP info: {:?}", ip_info);

    Ok(wifi)
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    wifi.set_configuration(&Configuration::AccessPoint(AccessPointConfiguration {
        ssid: heapless::String::try_from("BitClip").unwrap(),
        channel: 6,
        ..Default::default()
    }))?;

    wifi.start()?;

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    Ok(())
}

fn home_page() -> &'static str {
    return r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>File Directory</title>
</head>
<body>

<h1>File Directory</h1>

<ul>
    <li><a href="\#">Folder 1</a>
        <ul>
            <li><a href="\#">File A</a></li>
            <li><a href="\#">File B</a></li>
        </ul>
    </li>
    <li><a href="\#">Folder 2</a>
        <ul>
            <li><a href="\#">File C</a></li>
            <li><a href="\#">File D</a></li>
        </ul>
    </li>
    <li><a href="\#">File E</a></li>
</ul>

</body>
</html>
"#;
}
