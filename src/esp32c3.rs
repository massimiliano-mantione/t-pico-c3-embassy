#![allow(dead_code)]
#![allow(unused_variables)]

use core::{num::ParseIntError, str::FromStr};

use crate::uformat;
use crate::uformat::FormattedText;
use arrayvec::{ArrayString, ArrayVec, CapacityError};
use embassy_rp::{
    peripherals::{PIN_8, PIN_9, UART1},
    uart::{BufferedUart, BufferedUartTx, Config},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{with_timeout, Duration, Instant};
use embedded_io_async::{Read, Write};
use static_cell::make_static;

pub const HOSTNAME: &'static str = "countryman";
pub const MAX_NETWORK_BUFFER_SIZE: usize = 256;
pub type NetworkBuffer = ArrayVec<u8, MAX_NETWORK_BUFFER_SIZE>;

pub static OUT_DATA: Signal<CriticalSectionRawMutex, FormattedText> = Signal::new();

pub async fn esp32c3_task(uart1: UART1, pin_8: PIN_8, pin_9: PIN_9) {
    let tx_buf = &mut make_static!([0u8; 16])[..];
    let rx_buf = &mut make_static!([0u8; 16])[..];
    let uart = BufferedUart::new(
        uart1,
        super::Irqs,
        pin_8,
        pin_9,
        tx_buf,
        rx_buf,
        Config::default(),
    );
    let (mut rx, mut tx) = uart.split();

    let mut esp32c3 = Esp32C3::init();
    loop {
        let mut buf = [0u8];
        match with_timeout(
            Duration::from_secs(1),
            embassy_futures::select::select(rx.read_exact(&mut buf), OUT_DATA.wait()),
        )
        .await
        {
            Ok(s) => match s {
                embassy_futures::select::Either::First(_) => {
                    let received = buf[0];
                    esp32c3.update(Some(received), &mut tx).await;
                }
                embassy_futures::select::Either::Second(text) => {
                    esp32c3.send(&text, &mut tx).await;
                }
            },
            Err(_) => {
                esp32c3.update(None, &mut tx).await;
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CwState {
    Idle,
    ConnectedWithoutIpAddress,
    Connected,
    Connecting,
    Disconnected,
}

#[derive(Clone, Copy)]
enum IpSta {
    Ip(ArrayString<16>),
    Gateway(ArrayString<16>),
    Netmask(ArrayString<16>),
    Ip6Ll,
    Ip6Gl,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ConnectionIpState {
    link_id: u8,
    link_type: ArrayString<16>,
    remote_ip: ArrayString<16>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AccessPointEncryption {
    Open,
    Wep,
    WpaPsk,
    Wpa2Psk,
    WpaWpa2Psk,
    Wpa2Enterprise,
    Wpa3Psk,
    Wps2Wps3Psk,
    WapiPsk,
    Owe,
}

//+IPD,<length>:<data> will be outputted if AT+CIPMUX=0, AT+CIPRECVMODE=0, and AT+CIPDINFO=0.
//+IPD,<link_id>,<length>:<data> will be outputted if AT+CIPMUX=1, AT+CIPRECVMODE=0, and AT+CIPDINFO=0.
#[derive(Clone)]
struct IpData {
    #[allow(unused)]
    link_id: u8,
    #[allow(unused)]
    length: u16,
    data: NetworkBuffer,
}

// +CWLAP:<ecn>,<ssid>,<rssi>,<mac>,<channel>,<freq_offset>,<freqcal_val>,<pairwise_cipher>,<group_cipher>,<bgn>,<wps>
#[derive(Clone, Copy)]
pub struct AccessPointInfo {
    pub encryption: AccessPointEncryption,
    pub ssid: ArrayString<32>,
    pub rssi: u16,
}

#[derive(Clone, Copy)]
enum AddressInfo {
    AccessPointIp(ArrayString<16>),
    AccessPointIpV6Ll,
    AccessPointIpV6Gl,
    AccessPointMac(ArrayString<24>),
    StationIp(ArrayString<16>),
    StationIpV6Ll,
    StationIpV6Gl,
    StationMac(ArrayString<24>),
    EthIp,
    EthIpV6Ll,
    EthIpV6Gl,
    EthMac,
}

#[derive(Clone)]
enum AtReply {
    Empty,
    Ok,
    Error,
    WifiConnected,
    WifiGotIp,
    CwState(CwState),
    IpSta(IpSta),
    CIpState(ConnectionIpState),
    CWLAP(AccessPointInfo),
    SendOk,
    IpData(IpData),
    CIFSR(AddressInfo),
}

impl AtReply {
    pub fn description(&self) -> &'static str {
        match self {
            AtReply::Empty => "EMPTY",
            AtReply::Ok => "OK",
            AtReply::Error => "ERROR",
            AtReply::WifiConnected => "WIFI-CONNECTED",
            AtReply::WifiGotIp => "WIFI-GOT_IP",
            AtReply::CwState(_) => "CWSTATE",
            AtReply::IpSta(_) => "IPSTA",
            AtReply::CIpState(_) => "CIPSTATE",
            AtReply::CWLAP(_) => "CWLAP",
            AtReply::SendOk => "SEND-OK",
            AtReply::IpData(_) => "IPDATA",
            AtReply::CIFSR(_) => "CIFSR",
        }
    }
}

struct AtReplyParseError;
impl Into<AtReplyParseError> for ParseIntError {
    fn into(self) -> AtReplyParseError {
        AtReplyParseError
    }
}
impl Into<AtReplyParseError> for CapacityError<&str> {
    fn into(self) -> AtReplyParseError {
        AtReplyParseError
    }
}

impl AtReply {
    pub fn parse(s: &str, d: &[u8]) -> Self {
        Self::try_parse(s, d).unwrap_or(Self::Empty)
    }
    fn try_parse(s: &str, d: &[u8]) -> Result<Self, AtReplyParseError> {
        if s.len() == 0 {
            Ok(Self::Empty)
        } else if let Some(s) = s.strip_prefix("+") {
            if let Some(args) = s.strip_prefix("IPD,") {
                let (link_id, args) = args.split_once(",").ok_or(AtReplyParseError)?;
                let (length, data) = args.split_once(":").ok_or(AtReplyParseError)?;
                if data.len() > 0 {
                    return Ok(Self::Empty);
                }

                let link_id = link_id.parse::<u8>().map_err(|err| err.into())?;
                let length = length.parse::<u16>().map_err(|err| err.into())?;
                if length as usize != d.len() {
                    return Err(AtReplyParseError);
                }

                return Ok(Self::IpData(IpData {
                    link_id,
                    length,
                    data: ArrayVec::try_from(d).unwrap(),
                }));
            } else if let Some(args) = s.strip_prefix("CWSTATE:") {
                let state = args.split_once(",").map(|(s, _)| s).unwrap_or(args);
                let cw_state = match state {
                    "0" => CwState::Idle,
                    "1" => CwState::ConnectedWithoutIpAddress,
                    "2" => CwState::Connected,
                    "3" => CwState::Connecting,
                    "4" => CwState::Disconnected,
                    _ => return Ok(Self::Empty),
                };
                Ok(Self::CwState(cw_state))
            } else if let Some(args) = s.strip_prefix("CIPSTATE:") {
                let (link_id, args) = args.split_once(",").ok_or(AtReplyParseError)?;
                let (link_type, args) = args.split_once(",").ok_or(AtReplyParseError)?;
                let (remote_ip, _) = args.split_once(",").ok_or(AtReplyParseError)?;

                let link_id = link_id.parse::<u8>().map_err(|err| err.into())?;
                let link_type = ArrayString::try_from(link_type).map_err(|err| err.into())?;
                let remote_ip = ArrayString::try_from(remote_ip).map_err(|err| err.into())?;
                Ok(Self::CIpState(ConnectionIpState {
                    link_id,
                    link_type,
                    remote_ip,
                }))
            } else if let Some(args) = s.strip_prefix("CWLAP:") {
                let (enc, args) = args.split_once(",").ok_or(AtReplyParseError)?;
                let (ssid, args) = args.split_once(",").ok_or(AtReplyParseError)?;
                let (rssi, _) = args.split_once(",").ok_or(AtReplyParseError)?;
                let encryption = match enc {
                    "0" => AccessPointEncryption::Open,
                    "1" => AccessPointEncryption::Wep,
                    "2" => AccessPointEncryption::WpaPsk,
                    "3" => AccessPointEncryption::Wpa2Psk,
                    "4" => AccessPointEncryption::WpaWpa2Psk,
                    "5" => AccessPointEncryption::Wpa2Enterprise,
                    "6" => AccessPointEncryption::Wpa3Psk,
                    "7" => AccessPointEncryption::WapiPsk,
                    "8" => AccessPointEncryption::Owe,
                    _ => return Ok(Self::Empty),
                };
                let ssid = ArrayString::try_from(ssid).unwrap();
                let rssi = rssi.parse::<u16>().map_err(|err| err.into())?;
                Ok(Self::CWLAP(AccessPointInfo {
                    encryption,
                    ssid,
                    rssi,
                }))
            } else if let Some(args) = s.strip_prefix("CIPSTA:") {
                let (param, arg) = args.split_once(":").ok_or(AtReplyParseError)?;
                let info = match param {
                    "ip" => IpSta::Ip(ArrayString::try_from(arg).map_err(|_| AtReplyParseError)?),
                    "gateway" => {
                        IpSta::Gateway(ArrayString::try_from(arg).map_err(|_| AtReplyParseError)?)
                    }
                    "netmask" => {
                        IpSta::Netmask(ArrayString::try_from(arg).map_err(|_| AtReplyParseError)?)
                    }
                    "ip6ll" => IpSta::Ip6Ll,
                    "ip6gl" => IpSta::Ip6Gl,
                    _ => return Ok(Self::Empty),
                };
                Ok(Self::IpSta(info))
            } else if let Some(args) = s.strip_prefix("CIFSR:") {
                let (param, arg) = args.split_once(",").ok_or(AtReplyParseError)?;
                let info = match param {
                    "APIP" => AddressInfo::AccessPointIp(ArrayString::try_from(arg).unwrap()),
                    "APIP6LL" => AddressInfo::AccessPointIpV6Ll,
                    "APIP6GL" => AddressInfo::AccessPointIpV6Gl,
                    "APMAC" => AddressInfo::AccessPointMac(ArrayString::try_from(arg).unwrap()),
                    "STAIP" => AddressInfo::StationIp(ArrayString::try_from(arg).unwrap()),
                    "STAIP6LL" => AddressInfo::StationIpV6Ll,
                    "STAIP6GL" => AddressInfo::StationIpV6Gl,
                    "STAMAC" => AddressInfo::StationMac(ArrayString::try_from(arg).unwrap()),
                    "ETHIP" => AddressInfo::EthIp,
                    "ETHIP6LL" => AddressInfo::EthIpV6Ll,
                    "ETHIP6GL" => AddressInfo::EthIpV6Gl,
                    "ETHMAC" => AddressInfo::EthMac,
                    _ => return Ok(Self::Empty),
                };
                Ok(Self::CIFSR(info))
            } else {
                Ok(Self::Empty)
            }
        } else if s == "OK" {
            Ok(Self::Ok)
        } else if s == "SEND OK" {
            Ok(Self::SendOk)
        } else if s == "SEND ERROR" {
            Ok(Self::Error)
        } else if s == "WIFI CONNECTED" {
            Ok(Self::WifiConnected)
        } else if s == "WIFI GOT IP" {
            Ok(Self::WifiGotIp)
        } else if s.starts_with("ERROR") {
            Ok(Self::Error)
        } else {
            Ok(Self::Empty)
        }
    }
}

#[allow(unused)]
#[derive(Clone, Copy)]
enum CwMode {
    Null,
    Station,
    SoftAp,
    SoftApStation,
}

impl CwMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            CwMode::Null => "0",
            CwMode::Station => "1",
            CwMode::SoftAp => "2",
            CwMode::SoftApStation => "3",
        }
    }
}

#[derive(Clone, Copy)]
enum CIpRecvMode {
    Active,
    #[allow(unused)]
    Passive,
}

impl CIpRecvMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            CIpRecvMode::Active => "0",
            CIpRecvMode::Passive => "1",
        }
    }
}

#[derive(Clone)]
enum AtCommand {
    At,
    AtRst,
    AtRfPower,
    AtCwMode(CwMode),
    AtCIpRecvMode(CIpRecvMode),
    AtCwJap {
        ssid: ArrayString<32>,
        secret: ArrayString<32>,
    },
    ATCwReConnCfg {
        interval: u32,
        repeat: usize,
    },
    AtCwLap,
    AtCwAutoConn(bool),
    AtCwState,
    AtCwHostname(ArrayString<16>),
    AtCIpState,
    AtCIpSend {
        link_id: u8,
        data: NetworkBuffer,
    },
    #[allow(unused)]
    AtCIFSR,
    AtCIpMux(bool),
    AtCIpServCreate(u16),
    #[allow(unused)]
    AtCIpServShutdown(bool),
    AtCIpServMaxConn(u8),
    #[allow(unused)]
    AtCIpServTimeout(u16),
    AtCIpDInfo(bool),
    ATCIpTcpOpt {
        link_id: u8,
        so_linger: Option<u16>,
        tcp_nodelay: bool,
        so_sndtimeo: u16,
        keep_alive: u16,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RequiredNetworkStatus {
    Initial,
    NeedsRescan,
}

impl RequiredNetworkStatus {
    pub fn status(&self) -> NetworkStatus {
        match self {
            RequiredNetworkStatus::Initial => NetworkStatus::Initial,
            RequiredNetworkStatus::NeedsRescan => NetworkStatus::NeedsRescan,
        }
    }
}

const SECS_4: Duration = Duration::from_secs(4);
const SECS_5: Duration = Duration::from_secs(5);
const SECS_10: Duration = Duration::from_secs(10);
const NTW_INIT: RequiredNetworkStatus = RequiredNetworkStatus::Initial;
const NTW_SCAN: RequiredNetworkStatus = RequiredNetworkStatus::NeedsRescan;

impl AtCommand {
    pub fn timeout(&self) -> (Duration, RequiredNetworkStatus) {
        match self {
            AtCommand::At => (SECS_4, NTW_INIT),
            AtCommand::AtRst => (SECS_4, NTW_INIT),
            AtCommand::AtRfPower => (SECS_4, NTW_INIT),
            AtCommand::AtCwMode(_) => (SECS_4, NTW_INIT),
            AtCommand::AtCIpRecvMode(_) => (SECS_4, NTW_INIT),
            AtCommand::AtCwJap { .. } => (SECS_10, NTW_SCAN),
            AtCommand::ATCwReConnCfg { .. } => (SECS_4, NTW_INIT),
            AtCommand::AtCwLap => (SECS_10, NTW_SCAN),
            AtCommand::AtCwAutoConn(_) => (SECS_4, NTW_INIT),
            AtCommand::AtCwState => (SECS_4, NTW_INIT),
            AtCommand::AtCwHostname(_) => (SECS_4, NTW_INIT),
            AtCommand::AtCIpState => (SECS_4, NTW_INIT),
            AtCommand::AtCIpSend { .. } => (SECS_5, NTW_INIT),
            AtCommand::AtCIFSR => (SECS_4, NTW_INIT),
            AtCommand::AtCIpMux(_) => (SECS_4, NTW_INIT),
            AtCommand::AtCIpServCreate(_) => (SECS_4, NTW_INIT),
            AtCommand::AtCIpServShutdown(_) => (SECS_4, NTW_INIT),
            AtCommand::AtCIpServMaxConn(_) => (SECS_4, NTW_INIT),
            AtCommand::AtCIpServTimeout(_) => (SECS_4, NTW_INIT),
            AtCommand::AtCIpDInfo(_) => (SECS_4, NTW_INIT),
            AtCommand::ATCIpTcpOpt { .. } => (SECS_4, NTW_INIT),
        }
    }

    pub fn build(&self) -> ArrayString<MAX_NETWORK_BUFFER_SIZE> {
        match self {
            AtCommand::At => uformat!("ATE0\n\r").into(),
            AtCommand::AtRst => uformat!("AT+RST\n\r").into(),
            AtCommand::AtRfPower => uformat!("AT+RFPOWER=84\n\r").into(),
            AtCommand::AtCwMode(mode) => uformat!("AT+CWMODE={}\n\r", mode.as_str()).into(),
            AtCommand::AtCIpRecvMode(mode) => {
                uformat!("AT+CIPRECVMODE={}\n\r", mode.as_str()).into()
            }
            AtCommand::AtCwJap { ssid, secret } => {
                uformat!("AT+CWJAP=\"{}\",\"{}\"\n\r", ssid.as_str(), secret.as_str()).into()
            }
            AtCommand::ATCwReConnCfg { interval, repeat } => {
                uformat!("AT+CWRECONNCFG={},{}\n\r", *interval, *repeat).into()
            }
            AtCommand::AtCwLap => uformat!("AT+CWLAP\n\r").into(),
            AtCommand::AtCwAutoConn(autoconnect) => {
                uformat!("AT+CWAUTOCONN={}\n\r", if *autoconnect { "1" } else { "0" }).into()
            }
            AtCommand::AtCwState => uformat!("AT+CWSTATE?\n\r").into(),
            AtCommand::AtCwHostname(hostname) => {
                uformat!("AT+CWHOSTNAME=\"{}\"\n\r", hostname.as_str()).into()
            }
            AtCommand::AtCIpState => uformat!("AT+CIPSTA?\n\r").into(),
            AtCommand::AtCIpSend { link_id, data } => {
                let mut s: ArrayString<MAX_NETWORK_BUFFER_SIZE> =
                    uformat!("AT+CIPSEND:{},{}", link_id, data.len()).into();
                for b in data.iter().copied() {
                    s.push(b as char);
                }
                s.push('\n');
                s
            }
            AtCommand::AtCIFSR => uformat!("AT+CIFSR\n\r").into(),
            AtCommand::AtCIpMux(mux) => {
                uformat!("AT+CIPMUX={}\n\r", if *mux { "1" } else { "0" }).into()
            }
            AtCommand::AtCIpServCreate(port) => {
                uformat!("AT+CIPSERVER=1,{},\"TCP\",0\n\r", port).into()
            }
            AtCommand::AtCIpServShutdown(close_all) => {
                uformat!("ATCIPSERVER=0,{}\n\r", if *close_all { "1" } else { "0" }).into()
            }
            AtCommand::AtCIpServMaxConn(max) => uformat!("AT+CIPSERVERMAXCONN:{}\n\r", max).into(),
            AtCommand::AtCIpServTimeout(timeout) => uformat!("AT+CIPSTO:{}\n\r", timeout).into(),
            AtCommand::AtCIpDInfo(show) => {
                uformat!("AT+CIPDINFO:{}\n\r", if *show { "1" } else { "0" }).into()
            }
            AtCommand::ATCIpTcpOpt {
                link_id,
                so_linger,
                tcp_nodelay,
                so_sndtimeo,
                keep_alive,
            } => uformat!(
                "AT+CIPTCPOPT:{},{},{},{},{}\n\r",
                *link_id,
                if let Some(so_linger) = so_linger {
                    *so_linger as i16
                } else {
                    -1i16
                },
                if *tcp_nodelay { "1" } else { "0" },
                *so_sndtimeo,
                *keep_alive,
            )
            .into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Closed,
    SettingVerbosity,
    SettingIpOptions,
    Open,
}

#[derive(Clone, Copy)]
pub struct ConnectionInfo {
    #[allow(unused)]
    id: u8,
    status: ConnectionStatus,
}

pub enum NetworkStatus {
    Initial,
    Checking,
    SettingPower,
    SettingMode,
    SettingRecvMode,
    SettingReconnection,
    SettingAutoconnect,
    CheckingState,
    WaitingReconnection,
    ListingAp,
    JoiningAp { connected: bool, has_ip: bool },
    GettingIpAddress,
    SettingHostName,
    SettingMux,
    CreatingServer,
    SettingServerMaxConn,
    SettingConnectionVerbosity { link_id: u8 },
    SettingConnectionIpOptions { link_id: u8 },
    SendingData { link_id: u8, data: NetworkBuffer },
    Ready,
    NeedsReset,
    NeedsRescan,
    NeedsStateCheck,
    Resetting,
}

impl NetworkStatus {
    pub fn description(&self) -> &'static str {
        match self {
            NetworkStatus::Initial => "INITIAL",
            NetworkStatus::Checking => "CHECKING",
            NetworkStatus::SettingPower => "SETTING-POWER",
            NetworkStatus::SettingMode => "SETTING-MODE",
            NetworkStatus::SettingRecvMode => "SETTING-RECV-MODE",
            NetworkStatus::SettingReconnection => "SETTING-RECONNECTION",
            NetworkStatus::SettingAutoconnect => "SETTING-AUTOCONNECT",
            NetworkStatus::CheckingState => "CHECKING-STATE",
            NetworkStatus::WaitingReconnection => "WAITING-CONNECTION",
            NetworkStatus::ListingAp => "LISTING-AP",
            NetworkStatus::JoiningAp { connected, has_ip } => "JOINING-AP",
            NetworkStatus::GettingIpAddress => "GETTING-IP-ADDRESS",
            NetworkStatus::SettingHostName => "SETTING-HOST-NAME",
            NetworkStatus::SettingMux => "SETTING-MUX",
            NetworkStatus::CreatingServer => "CREATING-SERVER",
            NetworkStatus::SettingServerMaxConn => "SETTING-SERVER-MAX-CONN",
            NetworkStatus::SettingConnectionVerbosity { link_id } => "SETTING-CONNECTION-VERBOSITY",
            NetworkStatus::SettingConnectionIpOptions { link_id } => {
                "SETTING-CONNECTION-IP-OPTIONS"
            }
            NetworkStatus::SendingData { link_id, data } => "SENDING-DATA",
            NetworkStatus::Ready => "READY",
            NetworkStatus::NeedsReset => "NEEDS-RESET",
            NetworkStatus::NeedsRescan => "NEEDS-RESCAN",
            NetworkStatus::NeedsStateCheck => "NEEDS-STATE-CHECK",
            NetworkStatus::Resetting => "RESETTING",
        }
    }
}

const WIFI_SSID1: &'static str = include_str!("WIFI_SSID1.txt");
const WIFI_SECRET1: &'static str = include_str!("WIFI_SECRET1.txt");
const WIFI_SSID2: &'static str = include_str!("WIFI_SSID2.txt");
const WIFI_SECRET2: &'static str = include_str!("WIFI_SECRET2.txt");

const MAX_CONNECTIONS: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExpectedTermination {
    Newline,
    Bytes(usize),
}

impl ExpectedTermination {
    pub fn start() -> Self {
        ExpectedTermination::Newline
    }
}

pub struct Esp32C3 {
    current_ap_ssid: Option<ArrayString<32>>,
    current_ap_secret: Option<ArrayString<32>>,
    ip_address: Option<ArrayString<16>>,
    status: NetworkStatus,
    connections: [ConnectionInfo; MAX_CONNECTIONS],
    last_received: Option<NetworkBuffer>,
    current_reply: ArrayString<MAX_NETWORK_BUFFER_SIZE>,
    current_data: NetworkBuffer,
    expected_termination: ExpectedTermination,
    command_deadline: Instant,
    deadline_status: RequiredNetworkStatus,
}

impl Esp32C3 {
    pub fn init() -> Self {
        Self {
            current_ap_ssid: None,
            current_ap_secret: None,
            ip_address: None,
            status: NetworkStatus::Initial,
            connections: [
                ConnectionInfo {
                    id: 0,
                    status: ConnectionStatus::Closed,
                },
                ConnectionInfo {
                    id: 1,
                    status: ConnectionStatus::Closed,
                },
                ConnectionInfo {
                    id: 2,
                    status: ConnectionStatus::Closed,
                },
                ConnectionInfo {
                    id: 3,
                    status: ConnectionStatus::Closed,
                },
            ],
            last_received: None,
            current_reply: ArrayString::new(),
            current_data: ArrayVec::new(),
            expected_termination: ExpectedTermination::start(),
            command_deadline: Instant::now() + SECS_5,
            deadline_status: NTW_INIT,
        }
    }

    pub fn can_send(&self) -> bool {
        match &self.status {
            NetworkStatus::Ready => self
                .connections
                .iter()
                .any(|c| c.status == ConnectionStatus::Open),
            _ => false,
        }
    }

    async fn apply_command<'a>(&mut self, cmd: AtCommand, tx: &mut BufferedUartTx<'a, UART1>) {
        let (deadline, status) = cmd.timeout();
        let text = cmd.build();

        log::info!("APPLYING AT COMMAND: {}", text.as_str());

        self.command_deadline = Instant::now() + deadline;
        self.deadline_status = status;
        if let Err(err) = tx.write_all(text.as_bytes()).await {
            log::info!("Error writing to esp32c4: {}", err);
        }
    }

    pub async fn send<'a>(&mut self, text: &FormattedText, tx: &mut BufferedUartTx<'a, UART1>) {
        if let Some(cmd) = self.send_data(text.as_bytes()) {
            self.apply_command(cmd, tx).await;
        }
    }

    pub async fn update<'a>(&mut self, received: Option<u8>, tx: &mut BufferedUartTx<'a, UART1>) {
        if let Some(cmd) = self.auto_action() {
            log::info!("AUTOMATIC NETWORK ACTION");
            self.apply_command(cmd, tx).await;
            return;
        }

        if Instant::now() > self.command_deadline {
            log::info!("NETWORK TIMEOUT");
            self.status = self.deadline_status.status();
            return;
        }

        if let Some(byte) = received {
            let c = byte as char;

            let (reception_complete, expected_termination) = match (c, self.expected_termination) {
                (c, ExpectedTermination::Newline) => {
                    self.current_reply.push(c);

                    match c {
                        '\n' | '\r' => (true, self.expected_termination),
                        ':' => {
                            if let Some(ipd_args) = self.current_reply.as_str().strip_prefix("IPD,")
                            {
                                let length = match ipd_args.split_once(",") {
                                    Some((_link_id, length)) => length,
                                    None => {
                                        log::info!("received data, args ERROR");
                                        self.reset();
                                        return;
                                    }
                                };
                                let length = match length.parse::<usize>() {
                                    Ok(length) => length,
                                    Err(_) => {
                                        log::info!("received data, length ERROR");
                                        self.reset();
                                        return;
                                    }
                                };
                                self.current_data.clear();

                                log::info!("received data, got length");

                                if length > 0 {
                                    (false, ExpectedTermination::Bytes(length))
                                } else {
                                    (true, ExpectedTermination::Bytes(length))
                                }
                            } else {
                                (false, self.expected_termination)
                            }
                        }
                        _ => (false, self.expected_termination),
                    }
                }
                (_, ExpectedTermination::Bytes(mut remaining)) => {
                    remaining -= 1;
                    self.current_data.push(byte);
                    (remaining == 0, ExpectedTermination::Bytes(remaining))
                }
            };
            self.expected_termination = expected_termination;

            if reception_complete {
                log::info!("RECEPTION COMPLETE: '{}'", self.current_reply.as_str());

                if self.current_data.len() > 0 {
                    log::info!("DATA RECEIVED");
                    self.last_received = Some(self.current_data.clone());
                }
                let reply = AtReply::parse(self.current_reply.as_str(), &self.current_data);
                log::info!(
                    "REPLY RECEIVED: {} (STATUS {})",
                    reply.description(),
                    self.status.description()
                );
                self.current_reply.clear();
                let command = self.handle_reply(&reply);
                if let Some(cmd) = command {
                    self.apply_command(cmd, tx).await;
                }
            }
        }
    }

    fn reset_connections(&mut self) {
        self.connections.iter_mut().enumerate().for_each(|(id, c)| {
            *c = ConnectionInfo {
                id: id as u8,
                status: ConnectionStatus::Closed,
            }
        });
    }

    fn auto_action(&mut self) -> Option<AtCommand> {
        match &self.status {
            NetworkStatus::Initial => {
                self.status = NetworkStatus::Checking;
                Some(AtCommand::At)
            }
            NetworkStatus::NeedsReset => {
                self.status = NetworkStatus::Resetting;
                Some(AtCommand::AtRst)
            }
            NetworkStatus::NeedsRescan => {
                self.status = NetworkStatus::ListingAp;
                Some(AtCommand::AtCwLap)
            }
            NetworkStatus::NeedsStateCheck => {
                self.status = NetworkStatus::CheckingState;
                Some(AtCommand::AtCwState)
            }
            _ => None,
        }
    }

    fn reset(&mut self) -> Option<AtCommand> {
        self.reset_connections();
        self.status = NetworkStatus::NeedsReset;
        None
    }

    fn rescan(&mut self) -> Option<AtCommand> {
        self.reset_connections();
        self.status = NetworkStatus::NeedsRescan;
        None
    }

    #[allow(unused)]
    fn check_state(&mut self) -> Option<AtCommand> {
        self.reset_connections();
        self.status = NetworkStatus::NeedsStateCheck;
        None
    }

    fn setup_server(&mut self) -> Option<AtCommand> {
        self.status = NetworkStatus::GettingIpAddress;
        Some(AtCommand::AtCIpState)
    }

    fn setup_connection(&mut self, link_id: u8) -> Option<AtCommand> {
        self.status = NetworkStatus::SettingConnectionIpOptions { link_id };
        Some(AtCommand::AtCIpDInfo(false))
    }

    fn send_data(&mut self, data: &[u8]) -> Option<AtCommand> {
        if let NetworkStatus::Ready = self.status {
            for id in 0..MAX_CONNECTIONS {
                if self.connections[id].status == ConnectionStatus::Open {
                    let link_id = id as u8;
                    self.status = NetworkStatus::SendingData {
                        link_id,
                        data: data[0..MAX_NETWORK_BUFFER_SIZE].try_into().unwrap(),
                    };
                    return Some(AtCommand::AtCIpSend {
                        link_id,
                        data: data[0..MAX_NETWORK_BUFFER_SIZE].try_into().unwrap(),
                    });
                }
            }
        }
        None
    }

    fn send_data_to_next_connection(
        &mut self,
        next_candidate_connection: usize,
        data: NetworkBuffer,
    ) -> Option<AtCommand> {
        if next_candidate_connection < MAX_CONNECTIONS {
            for id in next_candidate_connection..MAX_CONNECTIONS {
                if self.connections[id].status == ConnectionStatus::Open {
                    let link_id = id as u8;
                    self.status = NetworkStatus::SendingData {
                        link_id,
                        data: data.clone(),
                    };
                    return Some(AtCommand::AtCIpSend { link_id, data });
                }
            }
        }
        None
    }

    fn handle_reply(&mut self, reply: &AtReply) -> Option<AtCommand> {
        match reply {
            AtReply::Empty => match &self.status {
                NetworkStatus::Checking => self.reset(),
                _ => None,
            },
            AtReply::Ok => match &self.status {
                NetworkStatus::Initial => None,
                NetworkStatus::Checking => {
                    self.status = NetworkStatus::SettingPower;
                    Some(AtCommand::AtRfPower)
                }
                NetworkStatus::SettingPower => {
                    self.status = NetworkStatus::SettingMode;
                    Some(AtCommand::AtCwMode(CwMode::Station))
                }
                NetworkStatus::SettingMode => {
                    self.status = NetworkStatus::SettingRecvMode;
                    Some(AtCommand::AtCIpRecvMode(CIpRecvMode::Active))
                }
                NetworkStatus::SettingRecvMode => {
                    self.status = NetworkStatus::SettingReconnection;
                    Some(AtCommand::ATCwReConnCfg {
                        interval: 5,
                        repeat: 10,
                    })
                }
                NetworkStatus::SettingReconnection => {
                    self.status = NetworkStatus::SettingAutoconnect;
                    Some(AtCommand::AtCwAutoConn(true))
                }
                NetworkStatus::SettingAutoconnect => {
                    self.status = NetworkStatus::ListingAp;
                    Some(AtCommand::AtCwLap)
                }
                NetworkStatus::CheckingState => self.rescan(),
                NetworkStatus::WaitingReconnection => self.rescan(),
                NetworkStatus::ListingAp => {
                    if let (Some(ssid), Some(secret)) = (
                        self.current_ap_ssid.as_ref(),
                        self.current_ap_secret.as_ref(),
                    ) {
                        self.status = NetworkStatus::JoiningAp {
                            connected: false,
                            has_ip: false,
                        };
                        Some(AtCommand::AtCwJap {
                            ssid: ArrayString::from(ssid).unwrap(),
                            secret: ArrayString::from(&secret).unwrap(),
                        })
                    } else {
                        self.rescan()
                    }
                }
                NetworkStatus::JoiningAp { connected, .. } => {
                    if *connected {
                        self.status = NetworkStatus::GettingIpAddress;
                        Some(AtCommand::AtCIFSR)
                    } else {
                        self.rescan()
                    }
                }
                NetworkStatus::GettingIpAddress => {
                    if self.ip_address.is_some() {
                        self.status = NetworkStatus::SettingHostName;
                        Some(AtCommand::AtCwHostname(
                            ArrayString::from(HOSTNAME).unwrap(),
                        ))
                    } else {
                        self.rescan()
                    }
                }
                NetworkStatus::SettingHostName => {
                    self.status = NetworkStatus::SettingMux;
                    Some(AtCommand::AtCIpMux(true))
                }
                NetworkStatus::SettingMux => {
                    self.status = NetworkStatus::SettingServerMaxConn;
                    Some(AtCommand::AtCIpServMaxConn(MAX_CONNECTIONS as u8))
                }
                NetworkStatus::SettingServerMaxConn => {
                    self.status = NetworkStatus::CreatingServer;
                    Some(AtCommand::AtCIpServCreate(3333))
                }
                NetworkStatus::CreatingServer => {
                    self.status = NetworkStatus::Ready;
                    None
                }
                NetworkStatus::SettingConnectionVerbosity { link_id } => {
                    let link_id = *link_id;
                    self.status = NetworkStatus::SettingConnectionIpOptions { link_id };
                    Some(AtCommand::ATCIpTcpOpt {
                        link_id,
                        so_linger: None,
                        tcp_nodelay: false,
                        so_sndtimeo: 0,
                        keep_alive: 0,
                    })
                }
                NetworkStatus::SettingConnectionIpOptions { .. } => {
                    self.status = NetworkStatus::Ready;
                    None
                }
                NetworkStatus::SendingData { link_id, data } => {
                    self.send_data_to_next_connection(*link_id as usize, data.clone())
                }
                NetworkStatus::Ready => None,
                NetworkStatus::NeedsReset => None,
                NetworkStatus::NeedsRescan => None,
                NetworkStatus::NeedsStateCheck => None,
                NetworkStatus::Resetting => {
                    self.status = NetworkStatus::Initial;
                    None
                }
            },
            AtReply::Error => match &self.status {
                NetworkStatus::Initial => self.reset(),
                NetworkStatus::Checking => self.reset(),
                NetworkStatus::SettingPower => self.reset(),
                NetworkStatus::SettingMode => self.reset(),
                NetworkStatus::SettingRecvMode => self.reset(),
                NetworkStatus::SettingReconnection => self.reset(),
                NetworkStatus::SettingAutoconnect => self.reset(),
                NetworkStatus::CheckingState => self.reset(),
                NetworkStatus::WaitingReconnection => self.reset(),
                NetworkStatus::ListingAp => self.rescan(),
                NetworkStatus::JoiningAp { .. } => self.rescan(),
                NetworkStatus::GettingIpAddress => self.rescan(),
                NetworkStatus::SettingHostName => self.rescan(),
                NetworkStatus::SettingMux => self.rescan(),
                NetworkStatus::CreatingServer => self.rescan(),
                NetworkStatus::SettingServerMaxConn => self.rescan(),
                NetworkStatus::SettingConnectionVerbosity { link_id } => {
                    self.connections[*link_id as usize].status = ConnectionStatus::Closed;
                    None
                }
                NetworkStatus::SettingConnectionIpOptions { link_id } => {
                    self.connections[*link_id as usize].status = ConnectionStatus::Closed;
                    None
                }
                NetworkStatus::SendingData { link_id, .. } => {
                    self.connections[*link_id as usize].status = ConnectionStatus::Closed;
                    None
                }
                NetworkStatus::Ready => self.reset(),
                NetworkStatus::NeedsReset => None,
                NetworkStatus::NeedsRescan => None,
                NetworkStatus::NeedsStateCheck => None,
                NetworkStatus::Resetting => None,
            },
            AtReply::WifiConnected => match &self.status {
                NetworkStatus::JoiningAp { has_ip, .. } => {
                    self.status = NetworkStatus::JoiningAp {
                        connected: true,
                        has_ip: *has_ip,
                    };
                    None
                }
                _ => {
                    self.status = NetworkStatus::JoiningAp {
                        connected: true,
                        has_ip: false,
                    };
                    None
                }
            },
            AtReply::WifiGotIp => match &self.status {
                NetworkStatus::JoiningAp { connected, .. } => {
                    self.status = NetworkStatus::JoiningAp {
                        connected: *connected,
                        has_ip: true,
                    };
                    None
                }
                _ => {
                    self.status = NetworkStatus::JoiningAp {
                        connected: true,
                        has_ip: true,
                    };
                    None
                }
            },
            AtReply::CwState(state) => match &self.status {
                NetworkStatus::CheckingState => match state {
                    CwState::Idle => self.rescan(),
                    CwState::ConnectedWithoutIpAddress => self.rescan(),
                    CwState::Connected => self.setup_server(),
                    CwState::Connecting => {
                        self.status = NetworkStatus::WaitingReconnection;
                        None
                    }
                    CwState::Disconnected => self.rescan(),
                },
                _ => self.rescan(),
            },
            AtReply::IpSta(stats) => {
                match stats {
                    IpSta::Ip(address) => {
                        self.ip_address = Some(address.clone());
                    }
                    _ => {}
                }
                None
            }
            AtReply::CIpState(connection_state) => self.setup_connection(connection_state.link_id),
            AtReply::CWLAP(ap) => {
                if ap.ssid.as_str() == WIFI_SSID1 {
                    self.current_ap_ssid = Some(ArrayString::from_str(WIFI_SSID1).unwrap());
                    self.current_ap_secret = Some(ArrayString::from_str(WIFI_SECRET1).unwrap());
                }
                if ap.ssid.as_str() == WIFI_SSID2 {
                    self.current_ap_ssid = Some(ArrayString::from_str(WIFI_SSID2).unwrap());
                    self.current_ap_secret = Some(ArrayString::from_str(WIFI_SECRET2).unwrap());
                }
                None
            }
            AtReply::SendOk => None,
            AtReply::IpData(received) => {
                self.last_received = Some(received.data.clone());
                None
            }
            AtReply::CIFSR(info) => {
                match info {
                    AddressInfo::StationIp(address) => self.ip_address = Some(address.clone()),
                    _ => {}
                }
                None
            }
        }
    }
}
