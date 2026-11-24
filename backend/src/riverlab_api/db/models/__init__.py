"""All ORM models. Importing this package registers every table on Base.metadata."""

from riverlab_api.db.models.academy import (
    AcademyActivity,
    AcademyAttempt,
    AcademyBookmark,
    AcademyLesson,
    AcademyLessonVersion,
    AcademyProgress,
    AcademyQuestion,
    AcademyReview,
)
from riverlab_api.db.models.config import GameConfig, Position, RakeConfig, StackConfig
from riverlab_api.db.models.goals import DailyGoals
from riverlab_api.db.models.history import HandHistory, PerformanceStat
from riverlab_api.db.models.play import HandReview, PlayHand, PlaySession
from riverlab_api.db.models.push_fold import PostflopAnswer, PushFoldAnswer
from riverlab_api.db.models.ranges import Range, RangeCombo
from riverlab_api.db.models.solver import (
    JobStatus,
    SavedSolution,
    SolverConfig,
    SolverJob,
    SolverResult,
    StrategyNode,
)
from riverlab_api.db.models.training import TrainingDecision, TrainingSession
from riverlab_api.db.models.users import User, UserSettings

__all__ = [
    "AcademyActivity",
    "AcademyAttempt",
    "AcademyBookmark",
    "AcademyLesson",
    "AcademyLessonVersion",
    "AcademyProgress",
    "AcademyQuestion",
    "AcademyReview",
    "DailyGoals",
    "GameConfig",
    "HandHistory",
    "HandReview",
    "JobStatus",
    "PerformanceStat",
    "PlayHand",
    "PlaySession",
    "Position",
    "PostflopAnswer",
    "PushFoldAnswer",
    "RakeConfig",
    "Range",
    "RangeCombo",
    "SavedSolution",
    "SolverConfig",
    "SolverJob",
    "SolverResult",
    "StackConfig",
    "StrategyNode",
    "TrainingDecision",
    "TrainingSession",
    "User",
    "UserSettings",
]
