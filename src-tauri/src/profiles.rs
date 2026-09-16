use crate::model::Profile;
pub fn minutes(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    if h < 24 && m < 60 {
        Some(h * 60 + m)
    } else {
        None
    }
}
pub fn in_time_range(now: u32, start: &str, end: &str) -> bool {
    match (minutes(start), minutes(end)) {
        (Some(a), Some(b)) if a <= b => now >= a && now < b,
        (Some(a), Some(b)) => now >= a || now < b,
        _ => false,
    }
}
pub fn wildcard(pattern: &str, value: &str) -> bool {
    let p = pattern.to_lowercase();
    let v = value.to_lowercase();
    let mut rest = v.as_str();
    let parts: Vec<_> = p.split('*').collect();
    if parts.len() == 1 {
        return p == v;
    }
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        let Some(pos) = rest.find(part) else {
            return false;
        };
        if i == 0 && pos != 0 {
            return false;
        }
        rest = &rest[pos + part.len()..];
    }
    p.ends_with('*') || rest.is_empty()
}
pub fn select<'a>(
    profiles: &'a [Profile],
    running: &[String],
    foreground: &str,
    minute: u32,
    idle: u32,
) -> Option<&'a Profile> {
    profiles
        .iter()
        .filter(|p| {
            let m = &p.r#match;
            m.processes.as_ref().is_none_or(|patterns| {
                patterns.iter().any(|pattern| {
                    if m.foreground_only.unwrap_or(false) {
                        wildcard(pattern, foreground)
                    } else {
                        running.iter().any(|r| wildcard(pattern, r))
                    }
                })
            }) && m
                .time_range
                .as_ref()
                .is_none_or(|r| in_time_range(minute, &r[0], &r[1]))
                && m.idle_minutes.is_none_or(|v| idle >= v)
        })
        .max_by_key(|p| {
            let m = &p.r#match;
            let category = if m.foreground_only.unwrap_or(false) {
                3
            } else if m.processes.is_some() {
                2
            } else if m.time_range.is_some() || m.idle_minutes.is_some() {
                1
            } else {
                0
            };
            (category, p.priority)
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn night_ranges_cross_midnight() {
        assert!(in_time_range(60, "23:00", "07:00"));
        assert!(!in_time_range(420, "23:00", "07:00"));
        assert!(!in_time_range(700, "23:00", "07:00"));
    }
    #[test]
    fn process_matching_is_case_insensitive() {
        assert!(wildcard("Ableton Live*.exe", "Ableton Live 12 Suite.exe"));
        assert!(wildcard("FL64.exe", "fl64.exe"));
        assert!(!wildcard("reaper.exe", "fake-reaper.exe"));
    }
    #[test]
    fn foreground_priority_wins() {
        use crate::model::*;
        let make = |id: &str, foreground, priority| Profile {
            id: id.into(),
            name: id.into(),
            r#match: ProfileMatch {
                processes: Some(vec!["DAW.exe".into()]),
                foreground_only: Some(foreground),
                ..Default::default()
            },
            preset_id: "aurora".into(),
            coexist_mode: CoexistMode::Handoff,
            priority,
        };
        let ps = vec![make("process", false, 999), make("foreground", true, 0)];
        assert_eq!(
            select(&ps, &["DAW.exe".into()], "DAW.exe", 60, 0)
                .unwrap()
                .id,
            "foreground"
        );
    }
}
