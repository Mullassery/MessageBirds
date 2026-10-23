"""Deprecated compatibility import for the pre-rebrand SDK name.

Use :mod:`pycustomerhub` instead. This module will be removed in a future
major release.
"""

from __future__ import annotations

import warnings

import pycustomerhub
from pycustomerhub import *  # noqa: F403
from pycustomerhub import PyCustomerHubClient

MessageBirdsClient = PyCustomerHubClient

__all__ = ["MessageBirdsClient", *pycustomerhub.__all__]

warnings.warn(
    "The 'messagebirds' Python package is deprecated; import 'pycustomerhub' instead.",
    DeprecationWarning,
    stacklevel=2,
)
