import { useEffect, useMemo, useRef, useState, useCallback } from 'react'
import ForceGraph2D from 'react-force-graph-2d'
import {
  getBrainGraph,
  queryBrain,
  searchBrain,
  type BrainGraphData,
  type BrainNode,
  type BrainRecommendation,
} from '../lib/api'
import { useWebSocket } from '../hooks/useWebSocket'

// Kind별 색상 매핑 - 마이너리티 리포트 스타일 네온 컬러
const KIND_COLORS: Record<string, string> = {
  user: '#00f5d4',           // 청록 - 사용자
  file: '#00bbf9',           // 시안 - 파일
  command: '#f8961e',        // 주황 - 명령어
  alert: '#f94144',          // 빨강 - 알림
  system: '#9b5de5',         // 본 - 시스템
  process: '#f15bb5',        // 핑크 - 프로세스
  network: '#3a86ff',        // 파랑 - 네트워크
  session: '#38b000',        // 초록 - 세션
  tool: '#ff6d00',           // 다크오렌지 - 도구
  project: '#7209b7',        // 볼 - 프로젝트
  incident: '#d00000',       // 진빨강 - 인시던트
  taskpattern: '#ff9e00',    // 오렌지 - 태스크 패턴
  decision: '#00f5d4',       // 청록 - 결정
  bottleneck: '#ff006e',     // 핫핑크 - 병목
  skill: '#06ffa5',          // 민트 - 스킬
  default: '#fee440',        // 노랑 - 기본
}

// Relation별 색상 매핑
const REL_COLORS: Record<string, string> = {
  executed: '#f8961e',
  accessed: '#00bbf9',
  modified: '#f94144',
  created: '#00f5d4',
  connected: '#9b5de5',
  triggered: '#f15bb5',
  uses: '#3a86ff',
  depends_on: '#ff6d00',
  produces: '#38b000',
  suggests_automation_for: '#ff9e00',
  default: '#ffffff',
}

// 노드 크기 매핑
const getNodeSize = (kind: string) => {
  switch (kind.toLowerCase()) {
    case 'alert':
    case 'incident':
      return 14
    case 'user':
    case 'system':
    case 'decision':
      return 11
    case 'command':
    case 'bottleneck':
      return 9
    case 'project':
    case 'session':
      return 10
    default:
      return 7
  }
}

interface GraphNode {
  id: string
  title: string
  kind: string
  x?: number
  y?: number
  vx?: number
  vy?: number
  fx?: number
  fy?: number
  val?: number
}

interface GraphLink {
  source: string | GraphNode
  target: string | GraphNode
  rel: string
  val?: number
}

export default function Brain() {
  const { events } = useWebSocket()
  const fgRef = useRef<any>(null)

  const [graph, setGraph] = useState<BrainGraphData | null>(null)
  const [recommendations, setRecommendations] = useState<BrainRecommendation[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [dimensions, setDimensions] = useState({ width: 1920, height: 1080 })

  const [keyword, setKeyword] = useState('')
  const [selectedKind, setSelectedKind] = useState('')
  const [selectedNode, setSelectedNode] = useState<BrainNode | null>(null)
  const [hoverNode, setHoverNode] = useState<GraphNode | null>(null)
  const [pulsingNodes, setPulsingNodes] = useState<Set<string>>(new Set())
  const [showLabels, setShowLabels] = useState(true)
  const [queryLoading, setQueryLoading] = useState(false)

  // Window dimensions
  useEffect(() => {
    const updateDimensions = () => {
      setDimensions({
        width: window.innerWidth,
        height: window.innerHeight,
      })
    }
    updateDimensions()
    window.addEventListener('resize', updateDimensions)
    return () => window.removeEventListener('resize', updateDimensions)
  }, [])

  // 그래프 데이터 변환
  const { nodes, links } = useMemo(() => {
    if (!graph) return { nodes: [], links: [] }

    const graphNodes: GraphNode[] = graph.nodes.map((n) => ({
      id: n.id,
      title: n.title,
      kind: n.kind,
      val: getNodeSize(n.kind),
    }))

    const graphLinks: GraphLink[] = graph.edges.map((e) => ({
      source: e.from,
      target: e.to,
      rel: e.rel,
      val: 1,
    }))

    return { nodes: graphNodes, links: graphLinks }
  }, [graph])

  // 초기 데이터 로드
  useEffect(() => {
    ;(async () => {
      try {
        setError(null)
        const [g, rec] = await Promise.all([getBrainGraph(), queryBrain('recommendations', 10)])
        setGraph(g)
        setRecommendations((rec.results as BrainRecommendation[]) ?? [])
      } catch (e) {
        setError((e as Error).message)
      } finally {
        setLoading(false)
      }
    })()
  }, [])

  // WebSocket 이벤트로 실시간 노드 pulse 효과
  useEffect(() => {
    if (!events.length || !graph) return

    const latestEvent = events[events.length - 1]
    const pulsing = new Set<string>()

    // 이벤트에서 관련된 노드 ID 추출
    graph.nodes.forEach((node) => {
      if (
        latestEvent.agent?.includes(node.id) ||
        node.title?.toLowerCase().includes(latestEvent.agent?.toLowerCase() || '')
      ) {
        pulsing.add(node.id)
      }
    })

    if (pulsing.size > 0) {
      setPulsingNodes(pulsing)
      setTimeout(() => setPulsingNodes(new Set()), 1000)
    }
  }, [events, graph])

  // 노드 검색
  const handleSearch = useCallback(async () => {
    if (!keyword.trim()) return
    setQueryLoading(true)
    try {
      const kindsParam = selectedKind.trim() ? [selectedKind.trim()] : undefined
      const res = await searchBrain(keyword, kindsParam, 30)
      setSelectedNode(res.results[0] ?? null)

      // 그래프에서 해당 노드 찾아서 줌
      if (res.results.length > 0) {
        const foundNode = res.results[0]
        const graphNode = nodes.find((n) => n.id === foundNode.id)
        if (graphNode && fgRef.current) {
          fgRef.current.centerAt(graphNode.x, graphNode.y, 1000)
          fgRef.current.zoom(2, 1000)
        }
      }
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setQueryLoading(false)
    }
  }, [keyword, selectedKind, nodes])

  // Kind 필터
  const handleKindFilter = useCallback(async (kind: string) => {
    setSelectedKind(kind)
    setKeyword('')
    setQueryLoading(true)
    try {
      const res = await searchBrain('', [kind], 30)
      setSelectedNode(res.results[0] ?? null)
      if (res.results.length > 0 && fgRef.current) {
        const graphNode = nodes.find((n) => n.id === res.results[0].id)
        if (graphNode) {
          fgRef.current.centerAt(graphNode.x, graphNode.y, 1000)
          fgRef.current.zoom(2, 1000)
        }
      }
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setQueryLoading(false)
    }
  }, [nodes])

  // 노드 그리기 콜백
  const nodeCanvasObject = useCallback(
    (node: any, ctx: CanvasRenderingContext2D, globalScale: number) => {
      const size = (node.val || 6) * (pulsingNodes.has(node.id) ? 1.5 : 1)
      const color = KIND_COLORS[node.kind.toLowerCase()] || KIND_COLORS.default
      const isSelected = selectedNode?.id === node.id
      const isHovered = hoverNode?.id === node.id
      const isConnected =
        selectedNode &&
        graph?.edges.some(
          (e) =>
            (e.from === selectedNode.id && e.to === node.id) ||
            (e.to === selectedNode.id && e.from === node.id)
        )

      // 글로우 효과
      if (isSelected || isHovered || pulsingNodes.has(node.id)) {
        ctx.shadowBlur = 25
        ctx.shadowColor = color
      } else if (isConnected) {
        ctx.shadowBlur = 12
        ctx.shadowColor = color
      } else {
        ctx.shadowBlur = 0
      }

      // 노드 본체
      ctx.beginPath()
      ctx.arc(node.x, node.y, size, 0, 2 * Math.PI)
      ctx.fillStyle = color
      ctx.fill()

      // 선택/호버 테두리
      if (isSelected || isHovered) {
        ctx.strokeStyle = '#ffffff'
        ctx.lineWidth = 2 / globalScale
        ctx.stroke()
      }

      // 내부 하이라이트 (3D 효과)
      ctx.beginPath()
      ctx.arc(node.x - size / 4, node.y - size / 4, size / 3, 0, 2 * Math.PI)
      ctx.fillStyle = 'rgba(255,255,255,0.4)'
      ctx.fill()

      ctx.shadowBlur = 0

      // 라벨
      if (showLabels || isSelected || isHovered) {
        const fontSize = isSelected ? 13 : 10
        ctx.font = `${isSelected ? 'bold' : 'normal'} ${fontSize}px Inter, system-ui, sans-serif`
        ctx.textAlign = 'center'
        ctx.textBaseline = 'middle'

        const text = node.title || node.id
        const metrics = ctx.measureText(text)
        const bgWidth = metrics.width + 10
        const bgHeight = fontSize + 6
        const labelX = node.x
        const labelY = node.y + size + 6

        // 라벨 배경 (둥근 사각형)
        ctx.fillStyle = 'rgba(0,0,0,0.75)'
        ctx.beginPath()
        const r = 4
        const x = labelX - bgWidth / 2
        const y = labelY
        ctx.moveTo(x + r, y)
        ctx.lineTo(x + bgWidth - r, y)
        ctx.quadraticCurveTo(x + bgWidth, y, x + bgWidth, y + r)
        ctx.lineTo(x + bgWidth, y + bgHeight - r)
        ctx.quadraticCurveTo(x + bgWidth, y + bgHeight, x + bgWidth - r, y + bgHeight)
        ctx.lineTo(x + r, y + bgHeight)
        ctx.quadraticCurveTo(x, y + bgHeight, x, y + bgHeight - r)
        ctx.lineTo(x, y + r)
        ctx.quadraticCurveTo(x, y, x + r, y)
        ctx.closePath()
        ctx.fill()

        // 라벨 텍스트
        ctx.fillStyle = isSelected ? '#ffffff' : 'rgba(255,255,255,0.85)'
        ctx.fillText(text, labelX, labelY + bgHeight / 2)
      }
    },
    [pulsingNodes, selectedNode, hoverNode, graph, showLabels]
  )

  // 링크 그리기 콜백
  const linkCanvasObject = useCallback(
    (link: any, ctx: CanvasRenderingContext2D) => {
      const start = link.source
      const end = link.target
      const rel = link.rel
      const color = REL_COLORS[rel] || REL_COLORS.default

      const isConnectedToSelected =
        selectedNode && (start.id === selectedNode.id || end.id === selectedNode.id)

      const lineWidth = isConnectedToSelected ? 2 : 0.5
      const opacity = isConnectedToSelected ? 1 : 0.25

      // 그라데이션 선
      const gradient = ctx.createLinearGradient(start.x, start.y, end.x, end.y)
      const alphaHex = Math.floor(opacity * 255)
        .toString(16)
        .padStart(2, '0')
      gradient.addColorStop(0, color + alphaHex)
      gradient.addColorStop(1, color + alphaHex)

      ctx.beginPath()
      ctx.moveTo(start.x, start.y)
      ctx.lineTo(end.x, end.y)
      ctx.strokeStyle = gradient
      ctx.lineWidth = lineWidth
      ctx.stroke()

      // 방향 표시 (화살표) - 선택된 경우만
      if (isConnectedToSelected) {
        const angle = Math.atan2(end.y - start.y, end.x - start.x)
        const arrowLength = 6
        const arrowAngle = Math.PI / 6
        const midX = (start.x + end.x) / 2
        const midY = (start.y + end.y) / 2

        ctx.beginPath()
        ctx.moveTo(midX, midY)
        ctx.lineTo(
          midX - arrowLength * Math.cos(angle - arrowAngle),
          midY - arrowLength * Math.sin(angle - arrowAngle)
        )
        ctx.moveTo(midX, midY)
        ctx.lineTo(
          midX - arrowLength * Math.cos(angle + arrowAngle),
          midY - arrowLength * Math.sin(angle + arrowAngle)
        )
        ctx.strokeStyle = color
        ctx.lineWidth = 1.5
        ctx.stroke()
      }
    },
    [selectedNode]
  )

  // 노드 클릭 핸들러
  const handleNodeClick = useCallback(
    (node: any) => {
      const brainNode = graph?.nodes.find((n) => n.id === node.id)
      if (brainNode) {
        setSelectedNode(brainNode)
      }
    },
    [graph]
  )

  // 연결된 엣지 필터링
  const connectedEdges = useMemo(() => {
    if (!selectedNode || !graph) return []
    return graph.edges.filter((e) => e.from === selectedNode.id || e.to === selectedNode.id)
  }, [selectedNode, graph])

  const connectedEdgesCount = connectedEdges.length

  // Kind 목록
  const kinds = useMemo(() => {
    return Object.keys(graph?.stats?.by_kind ?? {}).sort()
  }, [graph])

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen bg-black">
        <div className="text-center">
          <div className="w-16 h-16 border-4 border-blue-500/30 border-t-blue-500 rounded-full animate-spin mb-4" />
          <p className="text-blue-400 font-mono text-sm">INITIALIZING ONTOLOGY BRAIN...</p>
        </div>
      </div>
    )
  }

  // Debug info
  console.log('Brain render:', { nodes: nodes.length, links: links.length, dimensions, loading, error })

  return (
    <div className="relative w-full h-screen bg-black overflow-hidden">
      {/* 배경 그리드 */}
      <div
        className="absolute inset-0 opacity-20"
        style={{
          backgroundImage: `
            linear-gradient(rgba(0,185,249,0.1) 1px, transparent 1px),
            linear-gradient(90deg, rgba(0,185,249,0.1) 1px, transparent 1px)
          `,
          backgroundSize: '50px 50px',
        }}
      />

      {/* Debug Info Overlay */}
      <div className="absolute top-4 right-4 z-50 text-xs text-gray-500 font-mono bg-black/50 p-2 rounded">
        Nodes: {nodes.length} | Links: {links.length} | {dimensions.width}x{dimensions.height}
      </div>

      {/* Force Graph */}
      {nodes.length > 0 && (
        <ForceGraph2D
          ref={fgRef}
          graphData={{ nodes: nodes as any, links: links as any }}
          nodeCanvasObject={nodeCanvasObject}
          linkCanvasObject={linkCanvasObject}
          onNodeClick={handleNodeClick}
          onNodeHover={setHoverNode}
          backgroundColor="transparent"
          width={dimensions.width}
          height={dimensions.height}
          linkDirectionalArrowLength={0}
          linkDirectionalArrowRelPos={0.5}
          d3AlphaDecay={0.02}
          d3VelocityDecay={0.3}
          warmupTicks={100}
          cooldownTicks={50}
          nodeRelSize={6}
          linkWidth={1}
          linkColor={() => 'rgba(255,255,255,0.2)'}
        />
      )}

      {/* 좌측 상단: 검색 & 필터 패널 */}
      <div className="absolute top-4 left-4 w-80 bg-gray-950/90 backdrop-blur-md border border-gray-800 rounded-xl p-4 space-y-4 z-10">
        <div>
          <h1 className="text-xl font-bold text-white flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-blue-500 animate-pulse" />
            Ontology Brain
          </h1>
          <p className="text-xs text-gray-500 mt-1">
            {graph?.stats?.node_count?.toLocaleString()} nodes ·{' '}
            {graph?.stats?.edge_count?.toLocaleString()} edges
          </p>
        </div>

        {/* 검색 */}
        <div className="space-y-2">
          <div className="flex gap-2">
            <input
              className="flex-1 bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm text-white placeholder-gray-600 focus:border-blue-500 focus:outline-none transition-colors"
              placeholder="Search nodes..."
              value={keyword}
              onChange={(e) => setKeyword(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
            />
            <button
              onClick={handleSearch}
              disabled={queryLoading}
              className="bg-blue-600 hover:bg-blue-500 disabled:bg-blue-900 text-white text-sm px-4 py-2 rounded-lg transition-colors"
            >
              {queryLoading ? '...' : 'Find'}
            </button>
          </div>
        </div>

        {/* Kind 필터 */}
        <div>
          <p className="text-xs text-gray-500 mb-2">Filter by Kind</p>
          <div className="flex flex-wrap gap-1.5">
            {kinds.map((k) => (
              <button
                key={k}
                onClick={() => handleKindFilter(k)}
                className={`px-2.5 py-1 rounded-full text-xs border transition-all ${
                  selectedKind === k
                    ? 'border-blue-500 bg-blue-500/20 text-blue-300'
                    : 'border-gray-700 bg-gray-900 text-gray-400 hover:border-gray-500'
                }`}
                style={{
                  borderColor: selectedKind === k ? KIND_COLORS[k.toLowerCase()] : undefined,
                  color: selectedKind === k ? KIND_COLORS[k.toLowerCase()] : undefined,
                  backgroundColor: selectedKind === k ? `${KIND_COLORS[k.toLowerCase()]}20` : undefined,
                }}
              >
                {k}
              </button>
            ))}
          </div>
        </div>

        {/* 통계 */}
        <div className="grid grid-cols-3 gap-2 pt-2 border-t border-gray-800">
          {kinds.slice(0, 3).map((k) => (
            <div key={k} className="text-center">
              <p className="text-lg font-bold" style={{ color: KIND_COLORS[k.toLowerCase()] }}>
                {graph?.stats?.by_kind?.[k] ?? 0}
              </p>
              <p className="text-[10px] text-gray-500 uppercase">{k}</p>
            </div>
          ))}
        </div>

        {/* 컨트롤 */}
        <div className="flex items-center justify-between pt-2 border-t border-gray-800">
          <label className="flex items-center gap-2 text-xs text-gray-400 cursor-pointer">
            <input
              type="checkbox"
              checked={showLabels}
              onChange={(e) => setShowLabels(e.target.checked)}
              className="rounded border-gray-600"
            />
            Show Labels
          </label>
          <button
            onClick={() => fgRef.current?.zoomToFit(400)}
            className="text-xs text-blue-400 hover:text-blue-300"
          >
            Fit View
          </button>
        </div>
      </div>

      {/* 우측: 선택된 노드 상세 패널 */}
      <div
        className={`absolute top-4 right-4 w-80 bg-gray-950/95 backdrop-blur-md border border-gray-800 rounded-xl p-4 transition-all duration-300 z-10 ${
          selectedNode ? 'opacity-100 translate-x-0' : 'opacity-0 translate-x-4 pointer-events-none'
        }`}
      >
        {selectedNode ? (
          <div className="space-y-4">
            <div className="flex items-start justify-between">
              <div>
                <span
                  className="text-xs px-2 py-0.5 rounded-full border"
                  style={{
                    borderColor: KIND_COLORS[selectedNode.kind.toLowerCase()],
                    color: KIND_COLORS[selectedNode.kind.toLowerCase()],
                    backgroundColor: `${KIND_COLORS[selectedNode.kind.toLowerCase()]}15`,
                  }}
                >
                  {selectedNode.kind}
                </span>
                <h2 className="text-lg font-bold text-white mt-2">{selectedNode.title}</h2>
              </div>
              <button onClick={() => setSelectedNode(null)} className="text-gray-500 hover:text-white">
                ✕
              </button>
            </div>

            <div className="space-y-2">
              <p className="text-xs text-gray-500">ID</p>
              <p className="text-xs text-gray-300 font-mono break-all bg-gray-900 rounded p-2">
                {selectedNode.id}
              </p>
            </div>

            {/* 연결된 엣지 */}
            <div>
              <p className="text-xs text-gray-500 mb-2">Connected Relations ({connectedEdgesCount})</p>
              <div className="max-h-40 overflow-auto space-y-1">
                {connectedEdges.slice(0, 20).map((e, i) => (
                  <div key={i} className="flex items-center gap-2 text-xs p-2 rounded bg-gray-900/50">
                    <span className="text-gray-400 truncate flex-1">{e.from === selectedNode.id ? '→' : '←'}</span>
                    <span
                      className="px-1.5 py-0.5 rounded text-[10px]"
                      style={{
                        backgroundColor: `${REL_COLORS[e.rel]}30`,
                        color: REL_COLORS[e.rel],
                      }}
                    >
                      {e.rel}
                    </span>
                    <span className="text-gray-300 truncate flex-1">{e.from === selectedNode.id ? e.to : e.from}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        ) : (
          <p className="text-gray-500 text-sm">Select a node to view details</p>
        )}
      </div>

      {/* 하단: 추천 Priority */}
      {recommendations.length > 0 && (
        <div className="absolute bottom-4 left-4 right-4 flex justify-center">
          <div className="bg-gray-950/90 backdrop-blur-md border border-gray-800 rounded-xl p-3 max-w-4xl overflow-hidden">
            <div className="flex items-center gap-4">
              <span className="text-xs text-gray-500 whitespace-nowrap">RECOMMENDATIONS</span>
              <div className="flex gap-3 overflow-x-auto scrollbar-hide">
                {recommendations.slice(0, 5).map((r, i) => (
                  <div
                    key={i}
                    className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-gray-900/50 border border-gray-800 whitespace-nowrap"
                  >
                    <span
                      className="w-1.5 h-1.5 rounded-full"
                      style={{
                        backgroundColor:
                          r.priority === 'critical'
                            ? '#f94144'
                            : r.priority === 'high'
                              ? '#f8961e'
                              : '#00f5d4',
                      }}
                    />
                    <span className="text-xs text-gray-300">{r.title}</span>
                    <span className="text-[10px] text-gray-500">{r.score?.toFixed(1)}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* 에러 표시 */}
      {error && (
        <div className="absolute top-4 left-1/2 -translate-x-1/2 bg-red-500/10 border border-red-500/30 rounded-lg px-4 py-2">
          <p className="text-red-400 text-sm">{error}</p>
        </div>
      )}

      {/* 범례 */}
      <div className="absolute bottom-4 right-4 bg-gray-950/90 backdrop-blur-md border border-gray-800 rounded-xl p-3 z-10">
        <p className="text-xs text-gray-500 mb-2">Node Types</p>
        <div className="space-y-1.5">
          {Object.entries(KIND_COLORS)
            .filter(([k]) => !['default'].includes(k))
            .slice(0, 6)
            .map(([kind, color]) => (
              <div key={kind} className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full" style={{ backgroundColor: color }} />
                <span className="text-xs text-gray-400 capitalize">{kind}</span>
              </div>
            ))}
        </div>
      </div>
    </div>
  )
}
