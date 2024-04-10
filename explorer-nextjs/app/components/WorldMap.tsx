'use client'
/* eslint-disable @typescript-eslint/ban-ts-comment */

import * as React from 'react'
import { scaleLinear } from 'd3-scale'
import {
  ComposableMap,
  Geographies,
  Geography,
  Marker,
  ZoomableGroup,
} from 'react-simple-maps'
import ReactTooltip from 'react-tooltip'
import { CircularProgress } from '@mui/material'
import { useTheme } from '@mui/material/styles'
import { ApiState, CountryDataResponse } from '../typeDefs/explorer-api'
import MAP_TOPOJSON from '../assets/world-110m.json'

type MapProps = {
  userLocation?: [number, number]
  countryData?: ApiState<CountryDataResponse>
  loading: boolean
}

export const WorldMap: FCWithChildren<MapProps> = ({
  countryData,
  userLocation,
  loading,
}) => {
  const { palette } = useTheme()

  const colorScale = React.useMemo(() => {
    if (countryData?.data) {
      const heighestNumberOfNodes = Math.max(
        ...Object.values(countryData.data).map((country) => country.nodes)
      )
      return scaleLinear<string, string>()
        .domain([
          0,
          1,
          heighestNumberOfNodes / 4,
          heighestNumberOfNodes / 2,
          heighestNumberOfNodes,
        ])
        .range(palette.nym.networkExplorer.map.fills)
        .unknown(palette.nym.networkExplorer.map.fills[0])
    }
    return () => palette.nym.networkExplorer.map.fills[0]
  }, [countryData, palette])

  const [tooltipContent, setTooltipContent] = React.useState<string | null>(
    null
  )

  if (loading) {
    return <CircularProgress />
  }

  return (
    <>
      <ComposableMap
        data-tip=""
        style={{
          backgroundColor: palette.nym.networkExplorer.background.tertiary,
          width: '100%',
          height: 'auto',
        }}
        viewBox="0, 50, 800, 350"
        projection="geoMercator"
        projectionConfig={{
          scale: userLocation ? 200 : 100,
          center: userLocation,
        }}
      >
        <ZoomableGroup>
          <Geographies geography={MAP_TOPOJSON}>
            {({ geographies }) =>
              geographies.map((geo) => {
                const d = (countryData?.data || {})[geo.properties.ISO_A3]
                return (
                  <Geography
                    key={geo.rsmKey}
                    geography={geo}
                    fill={colorScale(d?.nodes || 0)}
                    stroke={palette.nym.networkExplorer.map.stroke}
                    strokeWidth={0.2}
                    onMouseEnter={() => {
                      const { NAME_LONG } = geo.properties
                      if (!userLocation) {
                        setTooltipContent(`${NAME_LONG} | ${d?.nodes || 0}`)
                      }
                    }}
                    onMouseLeave={() => {
                      setTooltipContent('')
                    }}
                    style={{
                      hover:
                        !userLocation && countryData
                          ? {
                              fill: palette.nym.highlight,
                              outline: 'white',
                            }
                          : undefined,
                    }}
                  />
                )
              })
            }
          </Geographies>

          {userLocation && (
            <Marker coordinates={userLocation}>
              <g
                fill="grey"
                stroke="#FF5533"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                transform="translate(-12, -10)"
              >
                <circle cx="12" cy="10" r="5" />
              </g>
            </Marker>
          )}
        </ZoomableGroup>
      </ComposableMap>
      <ReactTooltip>{tooltipContent}</ReactTooltip>
    </>
  )
}

WorldMap.defaultProps = {
  userLocation: undefined,
  countryData: undefined,
}
