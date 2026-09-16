package controllers

import "github.com/labstack/echo/v5"

func addUser(c *echo.Context) error {
	success(c, "successful")

	return nil
}

func UserRoutes(g *echo.Group) {
	r := g.Group("/user")

	r.POST("/add", addUser)
}
