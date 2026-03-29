---
name: weather
description: Looks up current weather for a city or location using a web search. Usage: /weather [city]
allowed-tools:
  - web_search
  - http_get
metadata:
  category: information
  execution: inline
---

# Weather

The user wants current weather information for a location.

## Usage

- `/weather London` → get weather for London
- `/weather` → ask the user for their location

## Instructions

1. If a city/location was provided after `/weather`, use it.
2. If not, ask the user for their location.
3. Use the web_search or http_get tool to look up current weather.
   - Try searching: "current weather in {city}" or use a weather API.
   - If no tool is available, provide a general description of how to check weather.

## Output format

**🌤 Weather for {city}**
- **Condition**: {sunny/cloudy/rainy/etc.}
- **Temperature**: {temp} ({°C or °F as appropriate for the region})
- **Humidity**: {%}
- **Wind**: {speed and direction}

{1-sentence summary or notable weather note}

If you cannot retrieve live weather data, clearly state that and suggest the user check weather.com, weather.gov, or their phone's weather app.
