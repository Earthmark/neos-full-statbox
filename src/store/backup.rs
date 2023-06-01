use super::RcStr;
use chrono::{DateTime, Utc};
use core::panic;
use serde::{
    de::{DeserializeOwned, Visitor},
    Deserialize, Serialize,
};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs::File,
    io::{self, BufReader, Read, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serde_json: {0} ({1})")]
    SerdeJson(serde_json::Error, PathBuf),
    #[error("Serde_bson: {0} ({1})")]
    SerdeBson(bson::de::Error, RcStr),
    #[error("Serde_bson_raw: {0} ({1})")]
    SerdeBsonRaw(bson::raw::Error, RcStr),
    #[error("Lzma: {0}")]
    Lzma(#[from] lzma_rs::error::Error),
}

fn os_to_cow(s: &OsStr) -> RcStr {
    s.to_string_lossy().into_owned().into()
}

trait FromDisk: Sized {
    fn from_disk(p: PathBuf) -> Result<Self, Error>;
}

impl<T: FromDisk> FromDisk for BTreeMap<RcStr, T> {
    fn from_disk(p: PathBuf) -> Result<Self, Error> {
        let dir = p.read_dir()?;
        let mut map = BTreeMap::<RcStr, T>::default();
        for dir in dir.into_iter() {
            let dir = dir?;
            if !dir
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".Storage.json")
            {
                let name = os_to_cow(&dir.path().file_stem().unwrap());
                let item = T::from_disk(dir.path())?;
                map.insert(name, item);
            }
        }
        Ok(map)
    }
}

impl<T: FromDisk> FromDisk for Vec<T> {
    fn from_disk(p: PathBuf) -> Result<Self, Error> {
        let dir = p.read_dir()?;
        let mut vec = Vec::<T>::default();
        for dir in dir.into_iter() {
            let dir = dir?;
            if !dir
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".Storage.json")
            {
                let item = T::from_disk(dir.path())?;
                vec.push(item);
            }
        }
        Ok(vec)
    }
}

trait FromFile: DeserializeOwned {}

impl<T> FromDisk for T
where
    T: FromFile,
{
    fn from_disk(p: PathBuf) -> Result<Self, Error> {
        from_file(p)
    }
}

fn from_file<T>(p: PathBuf) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let content = std::fs::File::open(&p)?;
    let buf_content = std::io::BufReader::new(content);
    let result = serde_json::from_reader(buf_content).map_err(|e| Error::SerdeJson(e, p))?;
    Ok(result)
}

impl FromDisk for Backup {
    fn from_disk(p: PathBuf) -> Result<Self, Error> {
        let mut backup = Self::default();

        for dir in p.read_dir()?.into_iter() {
            let dir = dir?;

            if dir.file_name() == "Assets" {
                backup.assets_dir = dir.path();
            } else {
                let (name, acc) = Account::load(dir.path())?;
                backup.accounts.insert(name, acc);
            }
        }
        Ok(backup)
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    #[serde(skip_serializing)]
    pub assets_dir: PathBuf,
    pub accounts: BTreeMap<RcStr, Account>,
}

impl Backup {
    pub fn load(root: PathBuf) -> Result<Self, Error> {
        Self::from_disk(root)
    }

    fn open_asset<P>(&self, id: P) -> Result<File, io::Error>
    where
        P: AsRef<Path>,
    {
        File::open(self.assets_dir.join(id))
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub contacts: BTreeMap<RcStr, Contact>,
    pub group_members: BTreeMap<RcStr, BTreeMap<RcStr, GroupMember>>,
    pub groups: BTreeMap<RcStr, Group>,
    pub messages: BTreeMap<RcStr, Vec<Message>>,
    pub records: BTreeMap<RcStr, Record>,
    pub variable_definitions: BTreeMap<RcStr, VariableDefinition>,
    pub variables: BTreeMap<RcStr, Variable>,
}

impl Account {
    fn load(root: PathBuf) -> Result<(RcStr, Self), Error> {
        let name = os_to_cow(root.file_name().unwrap());
        let mut acc = Self::default();
        for dir in root.read_dir()?.into_iter() {
            let dir = dir?;
            match dir.file_name().to_str().unwrap() {
                "Contacts" => acc.contacts = BTreeMap::<RcStr, Contact>::from_disk(dir.path())?,
                "GroupMembers" => {
                    acc.group_members =
                        BTreeMap::<RcStr, BTreeMap<RcStr, GroupMember>>::from_disk(dir.path())?
                }
                "Groups" => acc.groups = BTreeMap::<RcStr, Group>::from_disk(dir.path())?,
                "Messages" => {
                    acc.messages = BTreeMap::<RcStr, Vec<Message>>::from_disk(dir.path())?
                }
                "Records" => acc.records = BTreeMap::<RcStr, Record>::from_disk(dir.path())?,
                "VariableDefinitions" => {
                    acc.variable_definitions =
                        BTreeMap::<RcStr, VariableDefinition>::from_disk(dir.path())?
                }
                "Variables" => acc.variables = BTreeMap::<RcStr, Variable>::from_disk(dir.path())?,
                _ => panic!("Unknown folder in backup area!"),
            }
        }
        Ok((name, acc))
    }
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    id: RcStr,
    owner_id: RcStr,
    friend_username: RcStr,
    alternate_usernames: Option<RcStr>,
    friend_status: RcStr,
    is_accepted: bool,
    user_status: ContactStatus,
    #[serde(deserialize_with = "super::de::err_to_none")]
    latest_message_time: Option<DateTime<Utc>>,
    profile: Option<Profile>,
}

impl FromFile for Contact {}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ContactStatus {
    online_status: RcStr,
    #[serde(deserialize_with = "super::de::err_to_none")]
    last_status_change: Option<DateTime<Utc>>,
    current_session_id: Option<RcStr>,
    current_session_access_level: i32,
    current_session_hidden: bool,
    current_hosting: bool,
    compatibility_hash: Option<RcStr>,
    neos_version: Option<RcStr>,
    #[serde(rename = "publicRSAKey")]
    public_rsa_key: Option<RsaKey>,
    output_device: RcStr,
    is_mobile: bool,
    #[serde(rename = "CurrentSession")]
    current_session: Option<Session>,
    active_sessions: Option<Vec<Session>>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct RsaKey {
    pub exponent: RcStr,
    pub modulus: RcStr,
    pub p: Option<RcStr>,
    pub q: Option<RcStr>,
    #[serde(rename = "DP")]
    pub dp: Option<RcStr>,
    #[serde(rename = "DQ")]
    pub dq: Option<RcStr>,
    pub inverse_q: Option<RcStr>,
    pub d: Option<RcStr>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub name: RcStr,
    pub description: Option<RcStr>,
    pub corresponding_world_id: Option<CorrespondingWorldId>,
    pub tags: Vec<RcStr>,
    pub session_id: RcStr,
    pub normalized_session_id: RcStr,
    pub host_user_id: RcStr,
    pub host_machine_id: RcStr,
    pub host_username: RcStr,
    pub compatibility_hash: RcStr,
    pub universe_id: Option<RcStr>,
    pub neos_version: RcStr,
    pub headless_host: bool,
    #[serde(rename = "sessionURLs")]
    pub session_urls: Vec<RcStr>,
    #[serde(deserialize_with = "super::de::null_to_default")]
    pub parent_session_ids: Vec<RcStr>,
    #[serde(deserialize_with = "super::de::null_to_default")]
    pub nested_session_ids: Vec<RcStr>,
    pub session_users: Vec<SessionUsers>,
    pub thumbnail: RcStr,
    pub joined_users: i32,
    pub active_users: i32,
    pub total_joined_users: i32,
    pub total_active_users: i32,
    pub max_users: i32,
    pub mobile_friendly: bool,
    pub session_begin_time: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
    pub away_since: Option<DateTime<Utc>>,
    pub access_level: RcStr,
    #[serde(rename = "HasEnded")]
    pub has_ended: bool,
    #[serde(rename = "IsValid")]
    pub is_valid: bool,
    // There are more :D
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct CorrespondingWorldId {
    record_id: RcStr,
    owner_id: RcStr,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionUsers {
    pub username: RcStr,
    #[serde(rename = "userID")]
    pub user_id: RcStr,
    pub is_present: bool,
    pub output_device: i32,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    icon_url: RcStr,
    background_url: Option<RcStr>,
    tagline: Option<RcStr>,
    description: Option<RcStr>,
    profile_world_url: Option<RcStr>,
    #[serde(deserialize_with = "super::de::null_to_default")]
    showcase_items: Vec<RcStr>,
    #[serde(deserialize_with = "super::de::null_to_default")]
    token_opt_out: Vec<RcStr>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GroupMember {
    id: RcStr,
    owner_id: RcStr,
    quota_bytes: i64,
    used_bytes: u64,
}

impl FromFile for GroupMember {}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: RcStr,
    pub admin_user_id: RcStr,
    pub name: RcStr,
    pub quota_bytes: u64,
    pub used_bytes: u64,
}

impl FromFile for Group {}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: RcStr,
    pub owner_id: RcStr,
    pub recipient_id: RcStr,
    pub message_type: MessageType,
    pub content: RcStr,
    pub send_time: DateTime<Utc>,
    pub last_update_time: DateTime<Utc>,
    pub read_time: Option<DateTime<Utc>>,
}

impl FromFile for Message {}

#[derive(Serialize, Deserialize, Debug, Default)]
pub enum MessageType {
    #[default]
    Object,
    Text,
    SessionInvite,
    Sound,
    CreditTransfer,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct VariableDefinition {
    pub definition_owner_id: RcStr,
    pub subpath: RcStr,
    pub variable_type: RcStr,
    pub default_value: Option<RcStr>,
    pub read_permissions: Vec<RcStr>,
    pub write_permissions: Vec<RcStr>,
    pub list_permissions: Vec<RcStr>,
}

impl FromFile for VariableDefinition {}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub owner_id: RcStr,
    pub path: RcStr,
    pub value: RcStr,
}

impl FromFile for Variable {}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum RecordType {
    Audio,
    Directory,
    Link,
    #[default]
    Object,
    Texture,
    World,
}

#[derive(Serialize, Debug, Clone)]
pub struct SZBson(pub RcStr);

impl<Output: DeserializeOwned> WellKnownAssetKind<Output> for SZBson {
    type ParserError = Error;

    fn open(&self, b: &Backup) -> Result<Output, Self::ParserError> {
        let asset = b.open_asset(self.0.as_ref())?;
        let mut content = Vec::new();
        uncompress_7z(asset, &mut content)?;
        let bson = bson::RawDocumentBuf::from_bytes(content)
            .map_err(|e| Error::SerdeBsonRaw(e, self.0.clone()))?;
        let bson = bson
            .to_document()
            .map_err(|e| Error::SerdeBsonRaw(e, self.0.clone()))?;
        Ok(bson::de::from_document(bson).map_err(|e| Error::SerdeBson(e, self.0.clone()))?)
    }
}

fn uncompress_7z<W>(file: File, out: &mut W) -> Result<(), lzma_rs::error::Error>
where
    W: Write,
{
    let mut file = BufReader::new(file);

    let mut status = [0; 1 + 4 + 8]; // flag, dict size, and uncompressed size
    file.read(&mut status)?;
    file.read(&mut [0; 8])?; // discard the compressed size (it is not expected).
    let mut file = status.chain(file);

    Ok(lzma_rs::lzma_decompress(&mut file, out)?)
}

#[derive(Serialize, Debug, Clone)]
pub struct Webp(pub RcStr);

#[derive(Serialize, Debug, Clone)]
pub struct Ogg(pub RcStr);

#[derive(Serialize, Debug, Clone)]
pub struct Unknown {
    pub kind: Option<RcStr>,
    pub id: RcStr,
}

pub trait WellKnownAssetKind<Output> {
    type ParserError;
    fn open(&self, b: &Backup) -> Result<Output, Self::ParserError>;
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NeosRecAsset {
    group_id: RcStr,
    asset_id: RcStr,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum AssetUri {
    SZBson(SZBson),
    Webp(Webp),
    Ogg(Ogg),
    Unknown(Unknown),
    NeosRec(NeosRecAsset),
}

impl<'de> Deserialize<'de> for AssetUri {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct AssetUriVisitor;

        impl<'de> Visitor<'de> for AssetUriVisitor {
            type Value = AssetUri;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("expected a url of neosrec or neosdb")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let mut sp = v.split(":///");
                let protocol = sp.next();
                let path = sp.next();

                if let (Some(protocol), Some(path)) = (protocol, path) {
                    match protocol {
                        "neosdb" => {
                            let mut tail = path.split(".");
                            let path = tail.next().unwrap();
                            let kind = tail.next();
                            Ok(match kind {
                                Some("7zbson") => AssetUri::SZBson(SZBson(path.to_owned().into())),
                                Some("webp") => AssetUri::Webp(Webp(path.to_owned().into())),
                                Some("ogg") => AssetUri::Ogg(Ogg(path.to_owned().into())),
                                kind => AssetUri::Unknown(Unknown {
                                    kind: kind.map(|k| k.to_owned().into()),
                                    id: path.to_owned().into(),
                                }),
                            })
                        }
                        "neosrec" => {
                            let mut tail = path.split("/");
                            let path = tail.next().unwrap();
                            let kind = tail.next().unwrap();
                            Ok(AssetUri::NeosRec(NeosRecAsset {
                                group_id: path.to_owned().into(),
                                asset_id: kind.to_owned().into(),
                            }))
                        }
                        _ => Err(serde::de::Error::custom("unknown asset protocol")),
                    }
                } else {
                    Err(serde::de::Error::custom(
                        "protocol url did not contain a :///",
                    ))
                }
            }
        }

        d.deserialize_str(AssetUriVisitor)
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: RcStr,
    pub owner_id: RcStr,
    pub asset_uri: Option<AssetUri>, // Directory has null
    pub global_version: i32,
    pub local_version: i32,
    pub last_modifying_user_id: RcStr,
    pub last_modifying_machine_id: Option<RcStr>,
    pub name: RcStr,
    pub description: Option<RcStr>, // this is never populated?
    pub record_type: RecordType,
    pub owner_name: RcStr,
    #[serde(deserialize_with = "super::de::null_to_default")]
    pub tags: Vec<RcStr>,
    #[serde(deserialize_with = "super::de::option_split_backslashes")]
    pub path: Vec<RcStr>,
    pub thumbnail_uri: Option<AssetUri>,
    #[serde(deserialize_with = "super::de::err_to_none")]
    pub last_modification_time: Option<DateTime<Utc>>,
    pub creation_time: Option<DateTime<Utc>>,
    pub first_publish_time: Option<DateTime<Utc>>,
    pub is_public: bool,
    pub is_for_patrons: bool,
    pub visits: i32,
    pub rating: i32,
    pub random_order: i32,
    #[serde(deserialize_with = "super::de::null_to_default")]
    pub submissions: Vec<Submission>,
    #[serde(deserialize_with = "super::de::null_to_default")]
    #[serde(rename = "neosDBmanifest")]
    pub neos_db_manifest: Vec<AssetRef>,
}

impl FromFile for Record {}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AssetRef {
    pub hash: RcStr,
    pub bytes: u64,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Submission {
    pub id: RcStr,
    pub owner_id: RcStr,
    pub target_record_id: RecordId,
    pub submission_time: DateTime<Utc>,
    pub submitted_by_id: RcStr,
    pub submitted_by_name: RcStr,
    pub featured: bool,
    pub featured_by_user_id: Option<RcStr>,
    pub featured_timestamp: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecordId {
    pub record_id: RcStr,
    pub owner_id: RcStr,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Manifest {
    pub object: Option<Slot>,
    pub assets: Option<Vec<Component>>,
    pub type_versions: BTreeMap<RcStr, i64>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Slot {
    #[serde(rename = "ID")]
    pub id: RcStr,
    pub components: Field<Vec<Component>>,
    #[serde(rename = "Persistent-ID")]
    pub persistent_id: Option<RcStr>,
    pub name: Field<Option<RcStr>>,
    pub tag: Field<Option<RcStr>>,
    pub active: Field<bool>,
    pub position: Field<FVec3>,
    pub rotation: Field<FQuat>,
    pub scale: Field<FVec3>,
    pub order_offset: Field<i64>,
    pub parent_reference: RcStr,
    pub children: Vec<Slot>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Component {
    #[serde(rename = "Type")]
    pub cs_type: RcStr,
    pub data: Data,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Data {
    #[serde(rename = "ID")]
    pub id: RcStr,
    #[serde(rename = "persistent-ID")]
    pub persistent_id: Option<RcStr>,
    pub update_order: Field<i64>,
    pub enabled: Field<bool>,
    #[serde(flatten)]
    pub fields: BTreeMap<RcStr, FieldValue>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum DataField {
    Field(Field<bson::Bson>),
    Reference(RcStr),
    Compound {
        #[serde(rename = "ID")]
        id: RcStr,
        #[serde(flatten)]
        fields: BTreeMap<RcStr, bson::Bson>,
    },
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Field<T> {
    #[serde(rename = "ID")]
    pub id: RcStr,
    pub data: T,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum FieldValue {
    Str(RcStr),
    Bool(bool),
    Int64(i64),
    FVec2(FVec2),
    FVec3(FVec3),
    FVec4(FVec4),
    Null(Option<()>),
    Dunno(bson::Bson),
}

type FVec2 = [f64; 2];
type FVec3 = [f64; 3];
type FVec4 = [f64; 4];
type FQuat = FVec4;

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct SimulationSpace {
    #[serde(rename = "ID")]
    pub id: RcStr,
    pub local_space: Field<Option<RcStr>>,
    pub use_parent_space: Field<bool>,
    pub override_root_space: Field<Option<RcStr>>,
}
