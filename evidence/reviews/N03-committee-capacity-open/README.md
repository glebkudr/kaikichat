# Committee capacity — open, not accepted

The current implementation and independently reviewed tests are preserved as a
development checkpoint. The maximum-capacity gate has **not** passed reliably.
No release or full V1 readiness is inferred from this checkpoint.

One R22 run passed the complete unchanged TCP64 gate (q43, withholding42,
all64 cold unseeded recovery and three final effects). The same run failed QUIC
with five missing routes at45 seconds. A later R23 diagnostic TCP run failed with
two missing routes and correlated real shared processing capacity refusals.
See the raw-result SHA256 values, retained summaries and diagnosis files.
Large raw traces remain in output/committee-capacity-planning, outside Git.

Both transports passed the independent service-announcement retry preflight,
including actual invalid signatures, Core rejection/acceptance, preserved expiry
and bounded5/10/20/5-second retry/reset. That preflight is not the full
genuine-spend service gate. R23 adds bounded public diagnostic events only.
R24 is an unreviewed draft parked outside test discovery; no R24 production
change exists. Critic files preserve decisions, including REVISE and superseded
passes. They approve tests, not the full product or failing capacity gate.

Current general regressions and actual native application checks are separately
recorded in ../V1-readiness-20260909/. Product priorities now proceed to agent
orders; see Docs/V1_READINESS_2026_09_09_RU.md for sources and the bounded revisit.
