package main

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestRun(t *testing.T) {
	err := run()
	assert.NoError(t, err, "run() should not return an error in its basic form")
}
