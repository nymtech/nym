module.exports = {
  preset: "ts-jest",
  testEnvironment: "node",
  reporters: ["default", "jest-junit"],
  collectCoverageFrom: ["src/**/*.ts"],
};
