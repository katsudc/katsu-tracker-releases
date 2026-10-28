//! Public agent metadata only: never reads or sends Riot account credentials.
use crate::constants::{AgentData, AppConstants, APP_CONSTANTS};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::Deserialize;
use std::{collections::HashSet, path::PathBuf, time::{Duration, Instant}};

static CATALOG: Lazy<RwLock<AppConstants>> = Lazy::new(|| RwLock::new(APP_CONSTANTS.clone()));
static REFRESH: tokio::sync::Mutex<Option<Instant>> = tokio::sync::Mutex::const_new(None);

#[derive(Deserialize)]
struct PublicAgent {
    uuid: String,
    #[serde(rename="displayName")]
    name: String,
    #[serde(rename="isPlayableCharacter", default)]
    playable: bool,
    #[serde(rename="backgroundGradientColors", default)]
    colors: Option<Vec<String>>,
}
#[derive(Deserialize)]
struct Response { status:u16, data:Vec<PublicAgent> }

fn validated(mut agent:AgentData)->Option<AgentData> {
    let id=uuid::Uuid::parse_str(&agent.uuid).ok()?;
    if id.is_nil() {return None;}
    agent.uuid=id.to_string();
    agent.name=agent.name.trim().replace('/', "").to_lowercase();
    if agent.name.is_empty() || agent.name.len()>60 || !agent.name.chars().all(|c|c.is_alphanumeric() || c=='-' || c==' ') {return None;}
    if agent.color.len()!=7 || !agent.color.starts_with('#') || !agent.color[1..].bytes().all(|c|c.is_ascii_hexdigit()) {
        agent.color="#82e0d0".into();
    }
    Some(agent)
}

fn merge(base:AppConstants, incoming:impl IntoIterator<Item=AgentData>)->AppConstants {
    let mut agents=base.agents;
    let mut ids:HashSet<_>=agents.iter().map(|a|a.uuid.to_lowercase()).collect();
    let mut names:HashSet<_>=agents.iter().map(|a|a.name.to_lowercase()).collect();
    let mut added=Vec::new();
    for agent in incoming.into_iter().filter_map(validated) {
        if ids.contains(&agent.uuid) || names.contains(&agent.name) {continue;}
        ids.insert(agent.uuid.clone());names.insert(agent.name.clone());added.push(agent);
    }
    added.sort_by(|a,b|a.name.cmp(&b.name));
    agents.extend(added);
    AppConstants {agents}
}

fn parse_public(text:&str)->Result<Vec<AgentData>,serde_json::Error> {
    let response:Response=serde_json::from_str(text)?;
    if response.status!=200 {return Ok(Vec::new());}
    Ok(response.data.into_iter().filter(|a|a.playable).map(|a| {
        let color=a.colors.and_then(|colors|colors.into_iter().next()).filter(|v|v.len()>=6 && v.is_ascii()).map(|v|format!("#{}",&v[..6])).unwrap_or_else(||"#82e0d0".into());
        AgentData {uuid:a.uuid,name:a.name,color}
    }).collect())
}

pub fn name(id:&str)->String {
    CATALOG.read().agents.iter().find(|a|a.uuid.eq_ignore_ascii_case(id)).map(|a|a.name.clone()).unwrap_or_default()
}
pub fn id(name:&str)->String {
    let name=name.replace('/', "").to_lowercase();
    CATALOG.read().agents.iter().find(|a|a.name==name).map(|a|a.uuid.clone()).unwrap_or_default()
}

pub async fn refresh(cache_path:Option<PathBuf>)->AppConstants {
    let mut last=REFRESH.lock().await;
    if last.is_some_and(|at|at.elapsed()<Duration::from_secs(15*60)) {return CATALOG.read().clone();}
    if last.is_none() {
        if let Some(path)=cache_path.as_ref() {
            if let Ok(text)=std::fs::read_to_string(path) {
                if let Ok(saved)=serde_json::from_str::<AppConstants>(&text) {
                    let merged=merge(CATALOG.read().clone(),saved.agents);
                    *CATALOG.write()=merged;
                }
            }
        }
    }
    *last=Some(Instant::now());
    let result=async {
        let client=reqwest::Client::builder().https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(6)).connect_timeout(Duration::from_secs(3)).build()?;
        client.get("https://valorant-api.com/v1/agents?isPlayableCharacter=true&language=en-US")
            .send().await?.error_for_status()?.text().await
    }.await;
    match result {
        Ok(text)=>match parse_public(&text) {
            Ok(incoming)=> {
                let merged=merge(CATALOG.read().clone(),incoming);
                tracing::info!("[Agents] Public catalog loaded: {} playable agents",merged.agents.len());
                *CATALOG.write()=merged.clone();
                if let Some(path)=cache_path {
                    if let Some(parent)=path.parent() {let _=std::fs::create_dir_all(parent);}
                    if let Ok(json)=serde_json::to_string(&merged) {let _=std::fs::write(path,json);}
                }
            },
            Err(_)=>tracing::warn!("[Agents] Invalid public catalog; using bundled/cached agents"),
        },
        Err(_)=>tracing::warn!("[Agents] Public catalog unavailable; using bundled/cached agents"),
    }
    CATALOG.read().clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    const FUTURE:&str="11111111-2222-4333-8444-555555555555";
    #[test] fn future_playable_agent_is_added_without_replacing_existing_choices() {
        let body=format!(r##"{{"status":200,"data":[{{"uuid":"{FUTURE}","displayName":"Future Agent","isPlayableCharacter":true,"backgroundGradientColors":["8245baff"]}},{{"uuid":"22222222-2222-4333-8444-555555555555","displayName":"NPC","isPlayableCharacter":false}}]}}"##);
        let current=APP_CONSTANTS.clone();let count=current.agents.len();
        let updated=merge(current.clone(),parse_public(&body).unwrap());
        assert_eq!(updated.agents.len(),count+1);
        assert_eq!(updated.agents[count].name,"future agent");assert_eq!(updated.agents[count].color,"#8245ba");
        for (old,new) in current.agents.iter().zip(&updated.agents) {assert_eq!(old.uuid,new.uuid);assert_eq!(old.name,new.name);}
        assert_eq!(merge(updated.clone(),parse_public(&body).unwrap()).agents.len(),count+1);
    }
    #[test] fn rejects_invalid_ids_and_name_collisions_and_preserves_cached_agents() {
        let base=APP_CONSTANTS.clone();let count=base.agents.len();
        let make=|uuid:&str,name:&str|AgentData {uuid:uuid.into(),name:name.into(),color:"invalid".into()};
        let updated=merge(base,[make("bad","future"),make(FUTURE,"Jett"),make(FUTURE,"Future")]);
        assert_eq!(updated.agents.len(),count+1);assert_eq!(updated.agents[count].color,"#82e0d0");
        let saved=serde_json::to_string(&updated).unwrap();
        let saved:AppConstants=serde_json::from_str(&saved).unwrap();
        assert_eq!(merge(APP_CONSTANTS.clone(),saved.agents).agents.len(),count+1);
        assert!(parse_public("not json").is_err());
        assert_eq!(merge(updated,Vec::new()).agents.len(),count+1);
    }
}
