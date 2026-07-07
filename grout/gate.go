// ABOUTME: Gate source side — brief-path derivation from a decision log and
// ABOUTME: the brief-frontmatter fields grout maps onto a gate row.

package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"gopkg.in/yaml.v3"
)

// gateInfo carries the brief frontmatter fields (gate.* and
// recommendation.verdict) plus the absolute decision-log path.
type gateInfo struct {
	LogPath     string
	Workflow    string
	Entity      string
	EntityTitle string
	Stage       string
	Round       int
	Verdict     string
}

// briefPathForLog inverts subspace's DecisionLogPath derivation
// (gate-foo.md → gate-foo.decisions.jsonl).
func briefPathForLog(logPath string) string {
	return strings.TrimSuffix(logPath, ".decisions.jsonl") + ".md"
}

// GateFromLog resolves the decision log to an absolute path, reads the
// sibling brief, and maps its frontmatter onto a gateInfo.
func GateFromLog(logPath string) (gateInfo, error) {
	abs, err := filepath.Abs(logPath)
	if err != nil {
		return gateInfo{}, err
	}
	brief, err := os.ReadFile(briefPathForLog(abs))
	if err != nil {
		return gateInfo{}, err
	}
	return gateFromBrief(brief, abs)
}

// gateFromBrief parses the brief's YAML frontmatter (pure given bytes).
func gateFromBrief(brief []byte, logAbs string) (gateInfo, error) {
	fm, err := frontmatter(brief)
	if err != nil {
		return gateInfo{}, fmt.Errorf("%s: %w", briefPathForLog(logAbs), err)
	}
	var b struct {
		Gate struct {
			Workflow    string `yaml:"workflow"`
			Entity      string `yaml:"entity"`
			EntityTitle string `yaml:"entity-title"`
			Stage       string `yaml:"stage"`
			Round       int    `yaml:"round"`
		} `yaml:"gate"`
		Recommendation struct {
			Verdict string `yaml:"verdict"`
		} `yaml:"recommendation"`
	}
	if err := yaml.Unmarshal(fm, &b); err != nil {
		return gateInfo{}, fmt.Errorf("%s: frontmatter: %w", briefPathForLog(logAbs), err)
	}
	return gateInfo{
		LogPath:     logAbs,
		Workflow:    b.Gate.Workflow,
		Entity:      b.Gate.Entity,
		EntityTitle: b.Gate.EntityTitle,
		Stage:       b.Gate.Stage,
		Round:       b.Gate.Round,
		Verdict:     b.Recommendation.Verdict,
	}, nil
}

// frontmatter returns the YAML between the leading "---" line and the
// closing "---" line.
func frontmatter(doc []byte) ([]byte, error) {
	const delim = "---\n"
	s := string(doc)
	if !strings.HasPrefix(s, delim) {
		return nil, fmt.Errorf("no frontmatter")
	}
	rest := s[len(delim):]
	end := strings.Index(rest, "\n---\n")
	if end < 0 {
		return nil, fmt.Errorf("unterminated frontmatter")
	}
	return []byte(rest[:end]), nil
}
