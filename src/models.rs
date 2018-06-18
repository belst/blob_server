use chrono::prelude::*;
use postgis::ewkb;
use serde::{Serialize, Serializer};
use uuid::Uuid;

pub type Username = String;

fn ser_point<S>(p: &Option<ewkb::Point>, ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(point) = p {
        #[derive(Serialize)]
        struct P {
            x: f64,
            y: f64,
        };

        (P {
            x: point.x,
            y: point.y,
        }).serialize(ser)
    } else {
        ser.serialize_none()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct User {
    pub username: Username,
    pub token: Uuid,
    #[serde(serialize_with = "ser_point")]
    pub last_location: Option<ewkb::Point>,
    pub last_online: DateTime<Utc>,
    pub completion: i32, // i32 because postgres doesnt have unsigned types
}

#[derive(Clone, Debug, Serialize)]
pub struct Friendship {
    pub source: Username,
    pub target: Username,
    pub accepted: Option<DateTime<Utc>>,
}
