//! Roblox property value types
//!
//! All property values are wrapped in a typed container for JSON serialization.
//! This ensures we preserve type information and can round-trip accurately.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A typed property value that can be serialized to JSON
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum PropertyValue {
    // Primitive types
    #[serde(rename = "bool")]
    Bool(bool),

    #[serde(rename = "int")]
    Int(i32),

    #[serde(rename = "int64")]
    Int64(i64),

    #[serde(rename = "float")]
    Float(f32),

    #[serde(rename = "double")]
    Double(f64),

    #[serde(rename = "string")]
    String(String),

    // Vector types
    #[serde(rename = "Vector2")]
    Vector2(Vector2),

    #[serde(rename = "Vector2int16")]
    Vector2int16(Vector2int16),

    #[serde(rename = "Vector3")]
    Vector3(Vector3),

    #[serde(rename = "Vector3int16")]
    Vector3int16(Vector3int16),

    // Transform types
    #[serde(rename = "CFrame")]
    CFrame(CFrame),

    // Color types
    #[serde(rename = "Color3")]
    Color3(Color3),

    #[serde(rename = "Color3uint8")]
    Color3uint8(Color3uint8),

    #[serde(rename = "BrickColor")]
    BrickColor(u32),

    // UI types
    #[serde(rename = "UDim")]
    UDim(UDim),

    #[serde(rename = "UDim2")]
    UDim2(UDim2),

    #[serde(rename = "Rect")]
    Rect(Rect),

    // Sequence types
    #[serde(rename = "NumberSequence")]
    NumberSequence(NumberSequence),

    #[serde(rename = "ColorSequence")]
    ColorSequence(ColorSequence),

    #[serde(rename = "NumberRange")]
    NumberRange(NumberRange),

    // Enum type
    #[serde(rename = "Enum")]
    Enum(EnumValue),

    // Reference types
    #[serde(rename = "Ref")]
    Ref(Option<Uuid>),

    #[serde(rename = "Content")]
    Content(String),

    // Binary types
    #[serde(rename = "BinaryString")]
    BinaryString(String), // Base64 encoded

    #[serde(rename = "SharedString")]
    SharedString(SharedStringRef),

    // Font type
    #[serde(rename = "Font")]
    Font(FontValue),

    // Face/Axes types
    #[serde(rename = "Faces")]
    Faces(FacesValue),

    #[serde(rename = "Axes")]
    Axes(AxesValue),

    // Physics types
    #[serde(rename = "PhysicalProperties")]
    PhysicalProperties(PhysicalPropertiesValue),

    #[serde(rename = "Ray")]
    Ray(RayValue),

    #[serde(rename = "Region3")]
    Region3(Region3Value),

    #[serde(rename = "Region3int16")]
    Region3int16(Region3int16Value),

    // Security token (for protected strings)
    #[serde(rename = "ProtectedString")]
    ProtectedString(String),

    // Optional/nullable wrapper
    #[serde(rename = "OptionalCFrame")]
    OptionalCFrame(Option<CFrame>),

    // Unique ID (new type in modern Roblox)
    #[serde(rename = "UniqueId")]
    UniqueId(String),

    // Security capabilities
    #[serde(rename = "SecurityCapabilities")]
    SecurityCapabilities(u64),
}

// === Vector Types ===

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vector2int16 {
    pub x: i16,
    pub y: i16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vector3int16 {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

// === Transform Types ===

/// CFrame (Coordinate Frame) - position + rotation matrix
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CFrame {
    pub position: [f32; 3],
    /// 3x3 rotation matrix stored as row-major array
    pub rotation: [f32; 9],
}

impl Default for CFrame {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        }
    }
}

// === Color Types ===

/// Color3 with components in 0-1 range
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Color3 {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// Color3uint8 with components in 0-255 range
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Color3uint8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

// === UI Types ===

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct UDim {
    pub scale: f32,
    pub offset: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct UDim2 {
    pub x: UDim,
    pub y: UDim,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Rect {
    pub min: Vector2,
    pub max: Vector2,
}

// === Sequence Types ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NumberSequence {
    pub keypoints: Vec<NumberSequenceKeypoint>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct NumberSequenceKeypoint {
    pub time: f32,
    pub value: f32,
    pub envelope: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColorSequence {
    pub keypoints: Vec<ColorSequenceKeypoint>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ColorSequenceKeypoint {
    pub time: f32,
    pub color: Color3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct NumberRange {
    pub min: f32,
    pub max: f32,
}

// === Enum Type ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnumValue {
    #[serde(rename = "enumType")]
    pub enum_type: String,
    pub value: String,
}

// === Reference Types ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharedStringRef {
    pub hash: String,
    pub file: Option<String>,
}

// === Font Type ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FontValue {
    pub family: String,
    pub weight: String,
    pub style: String,
}

// === Face/Axes Types ===

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FacesValue {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
    pub front: bool,
    pub back: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct AxesValue {
    pub x: bool,
    pub y: bool,
    pub z: bool,
}

// === Physics Types ===

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PhysicalPropertiesValue {
    pub density: f32,
    pub friction: f32,
    pub elasticity: f32,
    pub friction_weight: f32,
    pub elasticity_weight: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct RayValue {
    pub origin: Vector3,
    pub direction: Vector3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Region3Value {
    pub min: Vector3,
    pub max: Vector3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Region3int16Value {
    pub min: Vector3int16,
    pub max: Vector3int16,
}

// === Attribute Types (for Instance Attributes) ===

/// Attributes can only store a subset of property types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum AttributeValue {
    #[serde(rename = "bool")]
    Bool(bool),

    #[serde(rename = "number")]
    Number(f64),

    #[serde(rename = "string")]
    String(String),

    #[serde(rename = "Vector2")]
    Vector2(Vector2),

    #[serde(rename = "Vector3")]
    Vector3(Vector3),

    #[serde(rename = "CFrame")]
    CFrame(CFrame),

    #[serde(rename = "Color3")]
    Color3(Color3),

    #[serde(rename = "UDim")]
    UDim(UDim),

    #[serde(rename = "UDim2")]
    UDim2(UDim2),

    #[serde(rename = "NumberSequence")]
    NumberSequence(NumberSequence),

    #[serde(rename = "ColorSequence")]
    ColorSequence(ColorSequence),

    #[serde(rename = "NumberRange")]
    NumberRange(NumberRange),

    #[serde(rename = "Rect")]
    Rect(Rect),

    #[serde(rename = "BrickColor")]
    BrickColor(u32),

    #[serde(rename = "Font")]
    Font(FontValue),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector3_serialization() {
        let v = PropertyValue::Vector3(Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        });
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("Vector3"));

        let deserialized: PropertyValue = serde_json::from_str(&json).unwrap();
        assert_eq!(v, deserialized);
    }

    #[test]
    fn test_cframe_serialization() {
        let cf = PropertyValue::CFrame(CFrame::default());
        let json = serde_json::to_string_pretty(&cf).unwrap();
        println!("{}", json);

        let deserialized: PropertyValue = serde_json::from_str(&json).unwrap();
        assert_eq!(cf, deserialized);
    }

    #[test]
    fn test_enum_serialization() {
        let e = PropertyValue::Enum(EnumValue {
            enum_type: "Material".to_string(),
            value: "Plastic".to_string(),
        });
        let json = serde_json::to_string(&e).unwrap();

        let deserialized: PropertyValue = serde_json::from_str(&json).unwrap();
        assert_eq!(e, deserialized);
    }
}
