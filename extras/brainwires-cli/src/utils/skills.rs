use brainwires_skills::{SkillRegistry, SkillSource};
use crate::utils::paths::PlatformPaths;

pub fn discover_skills(registry: &mut SkillRegistry) -> anyhow::Result<()> {
    let mut paths = Vec::new();
    if let Ok(p) = PlatformPaths::personal_skills_dir() {
        paths.push((p, SkillSource::Personal));
    }
    if let Ok(p) = PlatformPaths::project_skills_dir() {
        paths.push((p, SkillSource::Project));
    }
    registry.discover_from(&paths)
}
