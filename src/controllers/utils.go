package controllers

import (
	"net/http"

	"github.com/labstack/echo/v5"
)

type SuccessResponse struct {
	Success bool `json:"success"`
	Data    any  `json:"data"`
}

type FailureResponse struct {
	Success bool    `json:"success"`
	Error   string  `json:"error"`
	Detail  *string `json:"detail"`
}

func success(c *echo.Context, data any) {
	c.JSON(http.StatusOK, SuccessResponse{
		Success: true,
		Data:    data,
	})
}

func fail(c *echo.Context, status int, error string, detail *string) {
	c.JSON(status, FailureResponse{
		Success: false,
		Error:   error,
		Detail:  detail,
	})
}
