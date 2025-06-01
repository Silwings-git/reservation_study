use std::{collections::HashMap, convert::Infallible, str::FromStr};

use chrono::{DateTime, Utc};
use regex::Regex;

#[derive(Debug, Clone)]
pub enum ReservationConflictInfo {
    Parsed(ReservationConflict),
    Unparsed(String),
}

#[derive(Debug, Clone)]
pub struct ReservationConflict {
    pub new: ReservationWindow,
    pub old: ReservationWindow,
}

#[derive(Debug, Clone)]
pub struct ReservationWindow {
    pub rid: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl FromStr for ReservationConflictInfo {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(conflict) = s.parse() {
            Ok(ReservationConflictInfo::Parsed(conflict))
        } else {
            Ok(ReservationConflictInfo::Unparsed(s.to_string()))
        }
    }
}

impl FromStr for ReservationConflict {
    type Err = ();

    // "Key (resource_id, timespan)=(ocean-view-room-713, [\"2022-12-26 22:00:00+00\",\"2022-12-30 19:00:00+00\")) conflicts with existing key (resource_id, timespan)=(ocean-view-room-713, [\"2022-12-25 22:00:00+00\",\"2022-12-28 19:00:00+00\"))."
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ParsedInfo::from_str(s)?.try_into()
    }
}

impl TryFrom<ParsedInfo> for ReservationConflict {
    type Error = ();

    fn try_from(value: ParsedInfo) -> Result<Self, Self::Error> {
        Ok(ReservationConflict {
            new: value.new.try_into()?,
            old: value.old.try_into()?,
        })
    }
}

#[derive(Debug)]
struct ParsedInfo {
    new: HashMap<String, String>,
    old: HashMap<String, String>,
}

impl FromStr for ParsedInfo {
    type Err = ();

    // "Key (resource_id, timespan)=(ocean-view-room-713, [\"2022-12-26 22:00:00+00\",\"2022-12-30 19:00:00+00\")) conflicts with existing key (resource_id, timespan)=(ocean-view-room-713, [\"2022-12-25 22:00:00+00\",\"2022-12-28 19:00:00+00\"))."
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r#"\((?P<k1>[a-zA-Z0-9_-]+)\s*,\s*(?P<k2>[a-zA-Z0-9_-]+)\)=\((?P<v1>[a-zA-Z0-9_-]+)\s*,\s*\[(?P<v2>[^\)\]]+)"#).unwrap();
        let mut maps = re
            .captures_iter(s)
            .take(2)
            .map(|caps| {
                let mut a = HashMap::new();
                a.insert(caps["k1"].to_string(), caps["v1"].to_string());
                a.insert(caps["k2"].to_string(), caps["v2"].to_string());
                a
            })
            .collect::<Vec<HashMap<String, String>>>();

        if maps.len() != 2 {
            return Err(());
        }

        let old = maps.remove(1);
        let new = maps.remove(0);

        Ok(ParsedInfo { new, old })

        // let [new, old] = <[_; 2]>::try_from(maps).map_err(|_| ())?;
        // Ok(ParsedInfo { new, old })
    }
}

impl TryFrom<HashMap<String, String>> for ReservationWindow {
    type Error = ();

    // "2022-12-26 22:00:00+00","2022-12-30 19:00:00+00"
    fn try_from(value: HashMap<String, String>) -> Result<Self, Self::Error> {
        const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S%#z";
        let rid = value.get("resource_id").ok_or(())?.to_string();
        let timespan = value.get("timespan").ok_or(())?.replace('"', "");
        let split = timespan.splitn(2, ',').collect::<Vec<&str>>();
        let start = DateTime::parse_from_str(split[0], TIME_FORMAT)
            .map_err(|_| ())?
            .with_timezone(&Utc);
        let end = DateTime::parse_from_str(split[1], TIME_FORMAT)
            .map_err(|_| ())?
            .with_timezone(&Utc);
        Ok(ReservationWindow { rid, start, end })
    }
}

#[cfg(test)]
mod tests {

    use chrono::{DateTime, Utc};

    use super::ReservationConflictInfo;
    const ERROR_MSG: &str = "Key (resource_id, timespan)=(ocean-view-room-713, [\"2022-12-26 22:00:00+00\",\"2022-12-30 19:00:00+00\")) conflicts with existing key (resource_id, timespan)=(ocean-view-room-713, [\"2022-12-25 22:00:00+00\",\"2022-12-28 19:00:00+00\")).";

    #[test]
    pub fn reservation_conflict_info_parse_should_work() {
        let info: ReservationConflictInfo = ERROR_MSG.parse().unwrap();
        if let ReservationConflictInfo::Parsed(info) = info {
            assert_eq!(info.new.rid, "ocean-view-room-713");
            assert_eq!(
                info.new.start,
                "2022-12-26 22:00:00+0000".parse::<DateTime<Utc>>().unwrap()
            );
            assert_eq!(
                info.new.end,
                "2022-12-30 19:00:00+0000".parse::<DateTime<Utc>>().unwrap()
            );
            assert_eq!(info.old.rid, "ocean-view-room-713");
            assert_eq!(
                info.old.start,
                "2022-12-25 22:00:00+0000".parse::<DateTime<Utc>>().unwrap()
            );
            assert_eq!(
                info.old.end,
                "2022-12-28 19:00:00+0000".parse::<DateTime<Utc>>().unwrap()
            );
        } else {
            assert!(false);
        }
    }
}
