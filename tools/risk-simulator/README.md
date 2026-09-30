# Committee risk models

From the project root, run `python3 tools/risk-simulator/risk.py` and provide one JSON request on stdin. Example:

```json
{"model":"committee","honestKeys":[100],"adversarialKeys":[100],"committeeSize":10,"quorum":7,"grindCandidates":null}
```

The result uses exact reduced fractions for uniform fixed-unit sampling without replacement. It reports structural quorum-intersection failure and availability assuming all honest committee members are online. These are conditional model results, not a deployment recommendation or probability that an implemented consensus protocol will be attacked. `grindCandidates:null` leaves the grinding bound unknown. Supplying a count is an assumption, not evidence about an actual sequencer.

The `quorum` model returns an explicit pair of conflicting signer sets when the chosen threshold cannot guarantee an honest intersection:

```json
{"model":"quorum","members":4,"byzantine":1,"quorum":2,"onlineHonest":2}
```

Increasing quorum to3 in that partition preserves the structural safety condition but cannot finalize without Byzantine participation. The model never lowers a quorum automatically.

Run tests with `python3 -m unittest discover -s tests/models -p 'test_*.py'`. The same tests run in `scripts/check.sh`. Saved real CLI requests/results are in `output/risk-models/committee-v1.json`; each request can be passed back to the CLI. Detailed fields, limits, assumptions, mathematical derivation and primary sources are in `spec/models/committee-risk-v1.md`.

No external Python packages are required. This tool does not read keys, contact the network, modify registry state, select live operators, grant spend authority or prove physical independence. Further F05 work includes full spend/MLS transition and handover models, resource/repair/subsidy accounting and review wash-trading economics.
