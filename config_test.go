package sushi

import (
	"runtime"
	"testing"

	"gopkg.in/yaml.v3"
)

func print_test_header() {
	println("************************************************************")
	pc, _, _, _ := runtime.Caller(1)
	fn := runtime.FuncForPC(pc)
	println("** " + fn.Name())
	println("************************************************************")
}

func Test_LoadConfigBytes(t *testing.T) {
	print_test_header()
	println("chargement de la configuration")
	b, err := LoadConfigBytes("sushi.sample.yml")

	if err != nil {
		t.Error(err)
	}

	var c []*Node

	err = yaml.Unmarshal(b, &c)
	if err != nil {
		t.Error(err)
	}

	println(c[0].Name)
	if c[0].Name != "server with nothing" {
		t.Error("La valeur trouvé dans la configuration n'est pas celle attendu")
	}

}
