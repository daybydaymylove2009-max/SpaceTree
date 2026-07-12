/**
 * 文件依赖关系分析器
 * @module composables/useFileDependencyAnalyzer
 * @description 分析文件之间的依赖关系，评估删除影响
 */

import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { FileInfo } from '../types'

// 依赖关系类型
export type DependencyType = 
  | 'reference'      // 被引用
  | 'include'        // 包含/导入
  | 'link'           // 链接
  | 'resource'       // 资源文件
  | 'config'         // 配置文件
  | 'data'           // 数据文件

// 文件依赖信息
export interface FileDependency {
  source: string      // 依赖源文件
  target: string      // 依赖目标文件
  type: DependencyType
  strength: number    // 依赖强度 0-1
}

// 文件影响分析结果
export interface FileImpact {
  file: FileInfo
  referencedBy: string[]      // 被哪些文件引用
  references: string[]        // 引用了哪些文件
  impactScore: number         // 影响分数 0-100
  deleteRisk: 'low' | 'medium' | 'high' | 'critical'
  affectedApplications: string[]  // 可能影响的应用程序
}

// 依赖图
export interface DependencyGraph {
  nodes: Map<string, FileInfo>
  edges: FileDependency[]
}

/**
 * 文件依赖分析器组合式函数
 */
export function useFileDependencyAnalyzer() {
  const isAnalyzing = ref(false)
  const dependencyGraph = ref<DependencyGraph>({
    nodes: new Map(),
    edges: []
  })

  // 文件扩展名到依赖类型的映射
  const extensionDependencyMap: Record<string, DependencyType[]> = {
    'html': ['resource', 'link'],
    'css': ['resource'],
    'js': ['include', 'reference'],
    'ts': ['include', 'reference'],
    'vue': ['include', 'resource'],
    'json': ['config', 'data'],
    'xml': ['config', 'data'],
    'md': ['reference'],
    'txt': ['data'],
    'sql': ['reference'],
    'py': ['include'],
    'java': ['include'],
    'cpp': ['include'],
    'h': ['include'],
    'hpp': ['include']
  }

  /**
   * 分析文件内容查找依赖
   */
  async function analyzeFileDependencies(file: FileInfo): Promise<string[]> {
    const dependencies: string[] = []
    
    // 只分析文本文件
    const textExtensions = ['html', 'css', 'js', 'ts', 'vue', 'json', 'xml', 'md', 'txt', 'sql', 'py', 'java', 'cpp', 'h', 'hpp']
    if (!textExtensions.includes(file.file_extension.toLowerCase())) {
      return dependencies
    }

    try {
      // 读取文件内容
      const content = await invoke('read_file_content', { path: file.path }) as string
      
      // 正则表达式匹配常见的依赖模式
      const patterns = [
        // HTML: src="..." href="..."
        /(?:src|href)=["']([^"']+)["']/gi,
        // CSS/JS: @import "..." 或 import "..."
        /@import\s+["']([^"']+)["']/gi,
        // JS/TS: import ... from "..."
        /import\s+.*?\s+from\s+["']([^"']+)["']/gi,
        // JS/TS: require("...")
        /require\s*\(\s*["']([^"']+)["']\s*\)/gi,
        // Python: import ... 或 from ... import
        /(?:from|import)\s+([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)/gi,
        // Java/C++: import ... 或 #include
        /#include\s+["'<]([^"'>]+)["'>]/gi,
        // 通用 URL/路径
        /(?:file|path|url)\s*[:=]\s*["']([^"']+)["']/gi
      ]

      for (const pattern of patterns) {
        let match
        while ((match = pattern.exec(content)) !== null) {
          const dep = match[1].trim()
          if (dep && !dep.startsWith('http') && !dep.startsWith('//')) {
            dependencies.push(dep)
          }
        }
      }
    } catch (error) {
      console.error(`分析文件依赖失败 ${file.path}:`, error)
    }

    return [...new Set(dependencies)] // 去重
  }

  /**
   * 构建依赖图
   */
  async function buildDependencyGraph(files: FileInfo[]): Promise<DependencyGraph> {
    isAnalyzing.value = true
    
    const graph: DependencyGraph = {
      nodes: new Map(),
      edges: []
    }

    // 添加节点
    for (const file of files) {
      graph.nodes.set(file.path, file)
    }

    // 分析每个文件的依赖
    for (const file of files) {
      const dependencies = await analyzeFileDependencies(file)
      
      for (const dep of dependencies) {
        // 尝试解析相对路径
        const resolvedPath = resolveDependencyPath(file.path, dep)
        
        // 检查依赖的文件是否在图中
        const targetFile = findFileByPath(graph.nodes, resolvedPath) || 
                          findFileByName(graph.nodes, dep)
        
        if (targetFile) {
          const depTypes = extensionDependencyMap[file.file_extension.toLowerCase()] || ['reference']
          
          graph.edges.push({
            source: file.path,
            target: targetFile.path,
            type: depTypes[0],
            strength: calculateDependencyStrength(file, targetFile)
          })
        }
      }
    }

    dependencyGraph.value = graph
    isAnalyzing.value = false
    
    return graph
  }

  /**
   * 解析依赖路径
   */
  function resolveDependencyPath(sourcePath: string, dependency: string): string {
    // 简化处理，实际应该使用完整的路径解析逻辑
    if (dependency.startsWith('./') || dependency.startsWith('../')) {
      const sourceDir = sourcePath.substring(0, sourcePath.lastIndexOf('\\'))
      // 这里简化处理，实际需要完整的路径解析
      return sourceDir + '\\' + dependency.replace(/\.\.\/|\.\//g, '')
    }
    return dependency
  }

  /**
   * 根据路径查找文件
   */
  function findFileByPath(nodes: Map<string, FileInfo>, path: string): FileInfo | undefined {
    // 精确匹配
    if (nodes.has(path)) {
      return nodes.get(path)
    }
    
    // 模糊匹配（处理扩展名）
    for (const [filePath, file] of nodes) {
      if (filePath.includes(path) || path.includes(file.filename)) {
        return file
      }
    }
    
    return undefined
  }

  /**
   * 根据文件名查找文件
   */
  function findFileByName(nodes: Map<string, FileInfo>, name: string): FileInfo | undefined {
    for (const file of nodes.values()) {
      if (file.filename === name || file.filename.startsWith(name + '.')) {
        return file
      }
    }
    return undefined
  }

  /**
   * 计算依赖强度
   */
  function calculateDependencyStrength(source: FileInfo, target: FileInfo): number {
    let strength = 0.5 // 基础强度

    // 同目录增加强度
    const sourceDir = source.path.substring(0, source.path.lastIndexOf('\\'))
    const targetDir = target.path.substring(0, target.path.lastIndexOf('\\'))
    if (sourceDir === targetDir) {
      strength += 0.2
    }

    // 文件类型匹配增加强度
    if (source.file_extension === target.file_extension) {
      strength += 0.1
    }

    return Math.min(1, strength)
  }

  /**
   * 分析文件删除影响
   */
  function analyzeDeleteImpact(file: FileInfo, graph: DependencyGraph): FileImpact {
    const referencedBy: string[] = []
    const references: string[] = []
    const affectedApplications: string[] = []

    // 查找引用该文件的文件
    for (const edge of graph.edges) {
      if (edge.target === file.path) {
        referencedBy.push(edge.source)
      }
      if (edge.source === file.path) {
        references.push(edge.target)
      }
    }

    // 计算影响分数
    let impactScore = 0
    
    // 被引用越多，影响越大
    impactScore += referencedBy.length * 10
    
    // 依赖强度加权
    for (const edge of graph.edges) {
      if (edge.target === file.path) {
        impactScore += edge.strength * 20
      }
    }

    // 文件类型影响
    const highImpactExtensions = ['json', 'xml', 'config', 'ini']
    if (highImpactExtensions.includes(file.file_extension.toLowerCase())) {
      impactScore += 15
      affectedApplications.push('配置系统')
    }

    // 确定风险等级
    let deleteRisk: FileImpact['deleteRisk']
    if (impactScore >= 50) {
      deleteRisk = 'critical'
    } else if (impactScore >= 30) {
      deleteRisk = 'high'
    } else if (impactScore >= 10) {
      deleteRisk = 'medium'
    } else {
      deleteRisk = 'low'
    }

    return {
      file,
      referencedBy,
      references,
      impactScore: Math.min(100, impactScore),
      deleteRisk,
      affectedApplications
    }
  }

  /**
   * 批量分析删除影响
   */
  async function analyzeBatchDeleteImpact(files: FileInfo[]): Promise<FileImpact[]> {
    const graph = await buildDependencyGraph(files)
    const impacts: FileImpact[] = []

    for (const file of files) {
      const impact = analyzeDeleteImpact(file, graph)
      impacts.push(impact)
    }

    // 按影响分数排序
    impacts.sort((a, b) => b.impactScore - a.impactScore)

    return impacts
  }

  /**
   * 生成影响报告
   */
  function generateImpactReport(impact: FileImpact): string {
    const lines = [
      `文件: ${impact.file.filename}`,
      `影响分数: ${impact.impactScore}/100`,
      `删除风险: ${impact.deleteRisk === 'critical' ? '极高' : impact.deleteRisk === 'high' ? '高' : impact.deleteRisk === 'medium' ? '中' : '低'}`,
      ''
    ]

    if (impact.referencedBy.length > 0) {
      lines.push(`被 ${impact.referencedBy.length} 个文件引用:`)
      impact.referencedBy.slice(0, 5).forEach(ref => {
        lines.push(`  - ${ref}`)
      })
      if (impact.referencedBy.length > 5) {
        lines.push(`  ... 还有 ${impact.referencedBy.length - 5} 个`)
      }
      lines.push('')
    }

    if (impact.affectedApplications.length > 0) {
      lines.push(`可能影响的应用: ${impact.affectedApplications.join(', ')}`)
    }

    return lines.join('\n')
  }

  return {
    isAnalyzing,
    dependencyGraph,
    analyzeFileDependencies,
    buildDependencyGraph,
    analyzeDeleteImpact,
    analyzeBatchDeleteImpact,
    generateImpactReport
  }
}
