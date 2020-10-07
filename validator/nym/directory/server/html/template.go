package html

import (
	"html/template"
	"io/ioutil"
)

// LoadTemplate loads templates embedded by go-assets-builder
func LoadTemplate() (*template.Template, error) {
	t := template.New("")
	for name, file := range Assets.Files {

		h, err := ioutil.ReadAll(file)
		if err != nil {
			return nil, err
		}
		t, err = t.New(name).Parse(string(h))
		if err != nil {
			return nil, err
		}
	}
	return t, nil
}
