use chrono::prelude::*;
use postgis::ewkb;
use serde::{Serialize, Serializer};
use uuid::Uuid;

pub type Username = String;

fn ser_point<S>(p: &ewkb::Point, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    #[derive(Serialize)]
    struct P {
        x: f64,
        y: f64,
    };

    (P { x: p.x, y: p.y }).serialize(ser)
}

#[derive(Clone, Debug, Serialize)]
pub struct User {
    pub username: Username,
    pub token: Uuid,
    #[serde(serialize_with = "ser_point")]
    pub last_location: ewkb::Point,
    pub last_online: DateTime<Utc>,
    pub completion: i32, // i32 because postgres doesnt have unsigned types
}

#[derive(Clone, Debug, Serialize)]
pub struct Friendship {
    pub source: Username,
    pub target: Username,
    pub accepted: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Nearby {
    pub username: Username,
    #[serde(serialize_with = "ser_point")]
    pub last_location: ewkb::Point,
    pub last_online: DateTime<Utc>,
}
