#!/bin/bash

# Build and start RideViz-RS with Docker

echo "🦀 Building RideViz-RS..."
docker-compose build

echo ""
echo "🚀 Starting RideViz-RS..."
docker-compose up
