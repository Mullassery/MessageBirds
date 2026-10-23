"""Deprecated compatibility import for the pre-rebrand client module."""

from pycustomerhub.client import ApiError, PyCustomerHubClient

MessageBirdsClient = PyCustomerHubClient

__all__ = ["ApiError", "PyCustomerHubClient", "MessageBirdsClient"]
