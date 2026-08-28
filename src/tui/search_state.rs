use crate::db::search::ThreadRoleFilter;

#[derive(PartialEq)]
pub(crate) enum PanelFocus {
    SessionList,
    Preview,
}

pub(crate) enum SearchMouseTarget {
    SessionList(Option<usize>),
    Preview,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum FilterFocus {
    Source,
    Project,
    Topology,
    Time,
    Sort,
}

impl FilterFocus {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Source => Self::Project,
            Self::Project => Self::Topology,
            Self::Topology => Self::Time,
            Self::Time => Self::Sort,
            Self::Sort => Self::Source,
        }
    }

    pub(crate) fn previous(self) -> Self {
        match self {
            Self::Source => Self::Sort,
            Self::Project => Self::Source,
            Self::Topology => Self::Project,
            Self::Time => Self::Topology,
            Self::Sort => Self::Time,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TopologyFilter {
    Primary,
    Subagents,
    All,
}

impl TopologyFilter {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Primary => Self::Subagents,
            Self::Subagents => Self::All,
            Self::All => Self::Primary,
        }
    }

    pub(crate) fn previous(self) -> Self {
        match self {
            Self::Primary => Self::All,
            Self::Subagents => Self::Primary,
            Self::All => Self::Subagents,
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Primary => "Primary",
            Self::Subagents => "Subagents",
            Self::All => "All",
        }
    }

    pub(crate) fn thread_role_filter(self) -> Option<ThreadRoleFilter> {
        match self {
            Self::Primary => Some(ThreadRoleFilter::TopLevel),
            Self::Subagents => Some(ThreadRoleFilter::Subagent),
            Self::All => None,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum SourcePickerRow {
    All,
    Source(usize),
}

#[derive(Clone, Copy)]
pub(crate) enum ProjectPickerRow {
    All,
    Project(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum SortOrder {
    Relevance,
    Newest,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_filter_cycles_in_both_directions() {
        assert_eq!(TopologyFilter::Primary.next(), TopologyFilter::Subagents);
        assert_eq!(TopologyFilter::Subagents.next(), TopologyFilter::All);
        assert_eq!(TopologyFilter::All.next(), TopologyFilter::Primary);
        assert_eq!(TopologyFilter::Primary.previous(), TopologyFilter::All);
    }

    #[test]
    fn primary_filter_keeps_unknown_roles_but_excludes_subagents() {
        assert_eq!(TopologyFilter::Primary.thread_role_filter(), Some(ThreadRoleFilter::TopLevel));
        assert_eq!(
            TopologyFilter::Subagents.thread_role_filter(),
            Some(ThreadRoleFilter::Subagent)
        );
        assert_eq!(TopologyFilter::All.thread_role_filter(), None);
    }
}
