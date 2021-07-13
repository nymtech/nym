Feature: Simple setup
  Scenario: Run 4 validators and upload a contract
    Given Docker images are built
    When  4 validators are up and running
    Then  upload contract
    And   cleanup environment
