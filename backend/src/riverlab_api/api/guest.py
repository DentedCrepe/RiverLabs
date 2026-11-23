"""Guest mode: someone else trains on your RiverLab without changing your statistics.

The web app sends `X-RiverLab-Guest: 1` while guest mode is on (a per-device switch). Answers
are still graded and games still played, but nothing is recorded towards your progress,
streak, goals, analytics or leaks. See docs/guest-mode.md.
"""

from typing import Annotated

from fastapi import Depends, Header

from riverlab_api.errors import ApiError


def guest_mode(x_riverlab_guest: Annotated[str | None, Header()] = None) -> bool:
    return x_riverlab_guest == "1"


Guest = Annotated[bool, Depends(guest_mode)]


class GuestForbiddenError(ApiError):
    status_code = 403
    code = "guest_mode"
