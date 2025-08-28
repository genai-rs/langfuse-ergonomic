//! Example of tracking a multi-step LLM chain with Langfuse

use langfuse_ergonomic::LangfuseClient;
use serde_json::json;
use chrono::Utc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    
    let client = LangfuseClient::from_env()?;
    
    // Create a trace for the entire chain
    let trace = client.trace()
        .name("multi-step-reasoning")
        .input(json!({
            "question": "What is 25% of 80, and how much would that cost if each unit is $3?"
        }))
        .user_id("user-789")
        .send()
        .await?;
    
    let trace_id = &trace.id;
    println!("📊 Started trace: {}", trace_id);
    
    // Step 1: Calculate percentage
    let step1_start = Utc::now();
    let step1_span = "step1-calculate";
    
    client.span(trace_id)
        .id(step1_span)
        .name("calculate-percentage")
        .start_time(step1_start)
        .input(json!({"calculation": "25% of 80"}))
        .send()
        .await?;
    
    // Simulate LLM call for calculation
    let calculation_result = client.generation(trace_id)
        .parent_observation_id(step1_span)
        .name("llm-math")
        .model("gpt-4")
        .input(json!({
            "prompt": "Calculate 25% of 80. Show your work."
        }))
        .output(json!({
            "response": "25% of 80 = 0.25 × 80 = 20"
        }))
        .tokens(20, 15)
        .send()
        .await?;
    
    // Complete step 1
    client.span(trace_id)
        .id(step1_span)
        .output(json!({"result": 20}))
        .end_time(Utc::now())
        .send()
        .await?;
    
    println!("✅ Step 1 complete: 25% of 80 = 20");
    
    // Step 2: Calculate cost
    let step2_start = Utc::now();
    let step2_span = "step2-cost";
    
    client.span(trace_id)
        .id(step2_span)
        .name("calculate-cost")
        .start_time(step2_start)
        .input(json!({
            "units": 20,
            "price_per_unit": 3
        }))
        .send()
        .await?;
    
    // Simulate LLM call for cost calculation
    client.generation(trace_id)
        .parent_observation_id(step2_span)
        .name("llm-cost-calculation")
        .model("gpt-4")
        .input(json!({
            "prompt": "If we have 20 units and each costs $3, what's the total?"
        }))
        .output(json!({
            "response": "20 units × $3/unit = $60 total"
        }))
        .tokens(25, 12)
        .send()
        .await?;
    
    // Complete step 2
    client.span(trace_id)
        .id(step2_span)
        .output(json!({"total_cost": 60}))
        .end_time(Utc::now())
        .send()
        .await?;
    
    println!("✅ Step 2 complete: 20 units × $3 = $60");
    
    // Step 3: Synthesize final answer
    let step3_span = "step3-synthesize";
    
    client.span(trace_id)
        .id(step3_span)
        .name("synthesize-answer")
        .input(json!({
            "percentage_result": 20,
            "total_cost": 60
        }))
        .send()
        .await?;
    
    let final_answer = "25% of 80 is 20, and at $3 per unit, the total cost would be $60.";
    
    client.generation(trace_id)
        .parent_observation_id(step3_span)
        .name("llm-synthesis")
        .model("gpt-4")
        .input(json!({
            "prompt": "Combine the results into a final answer"
        }))
        .output(json!({
            "response": final_answer
        }))
        .tokens(30, 20)
        .send()
        .await?;
    
    // Update the trace with the final output
    client.trace()
        .id(trace_id)
        .output(json!({
            "final_answer": final_answer,
            "breakdown": {
                "percentage_calculation": "25% of 80 = 20",
                "cost_calculation": "20 × $3 = $60"
            }
        }))
        .send()
        .await?;
    
    println!("✅ Final answer: {}", final_answer);
    
    // Add evaluation scores
    client.score(trace_id, "accuracy")
        .value(1.0)
        .comment("All calculations are correct")
        .send()
        .await?;
    
    client.score(trace_id, "reasoning_quality")
        .value(0.9)
        .comment("Clear step-by-step reasoning")
        .send()
        .await?;
    
    // Calculate total tokens used
    let total_prompt_tokens = 20 + 25 + 30;
    let total_completion_tokens = 15 + 12 + 20;
    
    client.score(trace_id, "efficiency")
        .value(0.8)
        .metadata(json!({
            "total_prompt_tokens": total_prompt_tokens,
            "total_completion_tokens": total_completion_tokens,
            "total_tokens": total_prompt_tokens + total_completion_tokens
        }))
        .send()
        .await?;
    
    println!("\n📈 Chain execution complete!");
    println!("   Total tokens used: {}", total_prompt_tokens + total_completion_tokens);
    println!("   View in Langfuse: https://cloud.langfuse.com/trace/{}", trace_id);
    
    Ok(())
}