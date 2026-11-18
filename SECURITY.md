# Security

The optional system daemon runs with `CAP_DAC_OVERRIDE`, `CAP_DAC_READ_SEARCH` and `CAP_SYS_RAWIO`
to read RAPL and PCI BAR0 registers (read-only). If you find a way to abuse it, or any other
vulnerability, please report it privately through the repository's "Report a vulnerability"
(GitHub security advisories) instead of a public issue.
