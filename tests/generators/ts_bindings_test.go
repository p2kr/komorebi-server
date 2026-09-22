package generators

import (
	"path/filepath"
	"testing"

	"github.com/gzuidhof/tygo/tygo"
	"github.com/stretchr/testify/assert"
)

//go:generate go test -v .
func TestGenerateTypes(t *testing.T) {
	outputPath, err := filepath.Abs("../../../komorebi-web/src/lib/models/bindings")
	// outputPath, err := filepath.Abs("./bindings")
	assert.NoError(t, err)

	println("outputting ts types at", outputPath)
	commonTypeMappings := map[string]string{
		"time.Time":  "string /* RFC3339 */",
		"uuid.UUID":  "string /* uuid */",
		"mapset.Set": "/* mapset.Set */ Array",
	}

	config := &tygo.Config{
		TypeMappings: commonTypeMappings,
		Packages: []*tygo.PackageConfig{
			{
				Path:       "komorebi-server/src/controllers",
				OutputPath: filepath.Join(outputPath, "controllers.ts"),
				EnumStyle:  "enum",
			},
			{
				Path:       "komorebi-server/src/dto",
				OutputPath: filepath.Join(outputPath, "dto.ts"),
				EnumStyle:  "enum",
			},
			{
				Path:       "komorebi-server/src/models",
				OutputPath: filepath.Join(outputPath, "models.ts"),
				EnumStyle:  "enum",
			},
		},
	}
	gen := tygo.New(config)
	err = gen.Generate()

	assert.NoError(t, err)
}
