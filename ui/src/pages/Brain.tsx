import { useEffect, useMemo, useState } from 'react'
import {
  getBrainGraph,
  queryBrain,
  searchBrain,
  type BrainGraphData,
  type BrainNode,
  type BrainRecommendation,
} from '../lib/api'

export default function Brain() {
  const [graph, setGraph] = useState<BrainGraphData | null>(null)
  const [recommendations, setRecommendations] = useState<BrainRecommendation[]>([])
  const [queryLoading, setQueryLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const [keyword, setKeyword] = useState('')
  const [kind, setKind] = useState('')
  const [searchResults, setSearchResults] = useState<BrainNode[]>([])

  useEffect(() => {
    ;(async () => {
      try {
        setError(null)
        const [g, rec] = await Promise.all([
          getBrainGraph(),
          queryBrain('recommendations', 10),
        ])
        setGraph(g)
        setRecommendations((rec.results as BrainRecommendation[]) ?? [])
      } catch (e) {
        setError((e as Error).message)
      }
    })()
  }, [])

  const topEdges = useMemo(() => graph?.edges.slice(0, 30) ?? [], [graph])

  const onSearch = async () => {
    setQueryLoading(true)
    try {
      const kinds = kind.trim() ? [kind.trim()] : undefined
      const res = await searchBrain(keyword, kinds, 30)
      setSearchResults(res.results)
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setQueryLoading(false)
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-white">Ontology Brain</h1>
        <p className="text-sm text-gray-400 mt-1">
          그래프 시각화(노드/엣지), 우선순위 추천, 키워드 검색을 한 화면에서 확인합니다.
        </p>
      </div>

      {error && (
        <div className="rounded-lg border border-red-500/30 bg-red-500/10 px-4 py-3 text-red-300 text-sm">
          {error}
        </div>
      )}

      <section className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card title="Nodes" value={String(graph?.stats?.node_count ?? 0)} />
        <Card title="Edges" value={String(graph?.stats?.edge_count ?? 0)} />
        <Card title="Kinds" value={String(Object.keys(graph?.stats?.by_kind ?? {}).length)} />
      </section>

      <section className="bg-gray-900 border border-gray-800 rounded-xl p-5">
        <h2 className="font-semibold text-white mb-3">Recommendation Priority</h2>
        <div className="space-y-3">
          {recommendations.map((r, idx) => (
            <div key={`${r.title}-${idx}`} className="rounded-lg border border-gray-800 bg-gray-950 p-3">
              <div className="flex items-center justify-between mb-1">
                <p className="text-sm text-white font-medium">{r.title}</p>
                <span className="text-xs px-2 py-1 rounded bg-blue-500/20 text-blue-300">
                  {r.priority} / score {r.score ?? '-'}
                </span>
              </div>
              <p className="text-xs text-gray-400">{r.rationale}</p>
            </div>
          ))}
        </div>
      </section>

      <section className="bg-gray-900 border border-gray-800 rounded-xl p-5">
        <h2 className="font-semibold text-white mb-3">Search Brain Nodes</h2>
        <div className="flex flex-col md:flex-row gap-3 mb-3">
          <input
            className="flex-1 bg-gray-950 border border-gray-700 rounded px-3 py-2 text-sm"
            placeholder="keyword (id/title)"
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
          />
          <input
            className="w-full md:w-52 bg-gray-950 border border-gray-700 rounded px-3 py-2 text-sm"
            placeholder="kind filter (optional)"
            value={kind}
            onChange={(e) => setKind(e.target.value)}
          />
          <button
            onClick={onSearch}
            disabled={queryLoading}
            className="bg-blue-600 hover:bg-blue-500 disabled:bg-blue-900 text-white text-sm px-4 py-2 rounded"
          >
            {queryLoading ? 'Searching...' : 'Search'}
          </button>
        </div>

        <div className="max-h-72 overflow-auto space-y-2">
          {searchResults.map((n) => (
            <div key={n.id} className="rounded border border-gray-800 bg-gray-950 px-3 py-2">
              <p className="text-sm text-white">{n.title}</p>
              <p className="text-xs text-gray-500">{n.kind} · {n.id}</p>
            </div>
          ))}
          {searchResults.length === 0 && (
            <p className="text-xs text-gray-500">검색 결과가 없습니다.</p>
          )}
        </div>
      </section>

      <section className="bg-gray-900 border border-gray-800 rounded-xl p-5">
        <h2 className="font-semibold text-white mb-3">Graph Edges (preview)</h2>
        <div className="max-h-80 overflow-auto space-y-2">
          {topEdges.map((e, idx) => (
            <div key={`${e.from}-${e.to}-${idx}`} className="text-xs text-gray-300 border border-gray-800 rounded px-3 py-2 bg-gray-950">
              <span className="text-blue-300">{e.from}</span>
              <span className="mx-2 text-gray-500">--{e.rel}--&gt;</span>
              <span className="text-emerald-300">{e.to}</span>
            </div>
          ))}
        </div>
      </section>
    </div>
  )
}

function Card({ title, value }: { title: string; value: string }) {
  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl p-4">
      <p className="text-xs text-gray-500 mb-1">{title}</p>
      <p className="text-2xl font-bold text-white">{value}</p>
    </div>
  )
}
